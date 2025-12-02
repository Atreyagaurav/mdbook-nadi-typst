use mdbook_renderer::book::{BookItem, Chapter};
use mdbook_renderer::RenderContext;
use pulldown_cmark::{Options, Parser};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

mod config;

fn main() -> anyhow::Result<()> {
    let mut stdin = std::io::stdin();
    let ctx = RenderContext::from_json(&mut stdin).unwrap();

    let cfg: config::Config = ctx.config.get("output.typst")?.unwrap_or_default();

    let _ = std::fs::create_dir_all(&ctx.destination);
    let book_path = ctx.destination.join("book.typ");

    let file = std::fs::File::create(book_path)?;
    let mut writer = std::io::BufWriter::new(file);
    writeln!(writer, "{}", cfg.prelude(&ctx.root)?)?;

    for section in ctx.book.items {
        write_bookitem(&mut writer, section, 0)?;
    }

    Ok(())
}

fn write_bookitem(
    writer: &mut BufWriter<File>,
    item: BookItem,
    level: usize,
) -> std::io::Result<()> {
    match item {
        BookItem::Separator => writeln!(writer, "\n#pagebreak()"),
        BookItem::PartTitle(title) => {
            writeln!(
                writer,
                "\n#pagebreak()\n#set page(header:[#h(1fr) {title}])\n#bookpart()[{title}]"
            )
        }
        BookItem::Chapter(chap) => {
            if let Some(num) = chap.number.clone() {
                writeln!(writer, "\n#heading(level:{})[{}]", num.len(), chap.name)?;
                write_chapter(writer, chap, num.len(), true)
            } else {
                writeln!(writer, "\n#unum_chap()[{}]", chap.name)?;
                write_chapter(writer, chap, level, false)
            }
        }
    }
}
fn write_chapter(
    writer: &mut BufWriter<File>,
    chapter: Chapter,
    mut level: usize,
    number: bool,
) -> std::io::Result<()> {
    // if the chapter content has multiple top level titles
    let top_titles = chapter
        .content
        .lines()
        .filter(|l| l.starts_with("# "))
        .count();
    let mut contents = chapter.content;
    if top_titles == 1 && contents.trim().starts_with('#') {
        level -= 1;
        contents = contents.lines().skip(1).collect::<Vec<&str>>().join("\n");
    }
    write_markdown(writer, contents, level, chapter.path, &chapter.name, number)?;

    for item in chapter.sub_items {
        write_bookitem(writer, item, level + 1)?;
    }
    writeln!(writer)
}

#[derive(Default)]
struct MdTable {
    aligns: Vec<&'static str>,
    headers: Vec<String>,
    on_cell: bool,
    thiscell: String,
    cells: Vec<String>,
}

fn write_markdown(
    writer: &mut BufWriter<File>,
    md: String,
    chap_level: usize,
    chap_path: Option<PathBuf>,
    chap_name: &str,
    number: bool,
) -> std::io::Result<()> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(&md, options);
    use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};

    let mut table: Option<MdTable> = None;
    let mut list: Option<u64> = None;
    let mut consec_par = false;
    let mut in_listitem = false;
    let mut in_code = false;
    let mut in_head = false;
    for event in parser {
        match event {
            Event::Code(c) => {
                if let Some(table) = &mut table {
                    table.thiscell.push_str(&format!("`{c}`"));
                } else {
                    write!(writer, "`{c}`")?
                }
            }
            Event::Text(c) => {
                let txt = if in_code {
                    let l = c
                        .lines()
                        .map(|l| l.trim_start_matches('!'))
                        .collect::<Vec<&str>>();
                    format!("{}\n", l.join("\n"))
                } else if in_head {
                    let cp = chap_path
                        .as_ref()
                        .and_then(|p| p.file_stem())
                        .map(|f| f.to_string_lossy());
                    maybe_label(cp.as_ref().map_or(chap_name, |v| &v), c)
                } else {
                    escape_typst(c)
                };
                if let Some(table) = &mut table {
                    table.thiscell.push_str(&txt);
                } else {
                    write!(writer, "{txt}")?
                }
            }
            Event::Html(html) => write!(writer, "{}", html_block(html))?,
            Event::SoftBreak => write!(writer, "\n")?,
            Event::HardBreak => write!(writer, "\n\n")?,
            // it makes four empty line, but overkill better than incorrect
            Event::Start(Tag::Paragraph) => {
                if !(in_listitem | consec_par) {
                    writeln!(writer, "\n\n")?
                }
            }
            Event::End(TagEnd::Paragraph) => {
                writeln!(writer, "\n")?;
                consec_par = true;
                continue;
            }
            Event::Start(Tag::Strong) => write!(writer, "*")?,
            Event::End(TagEnd::Strong) => write!(writer, "*")?,
            Event::Start(Tag::Link { dest_url, .. }) => {
                if let Some(table) = &mut table {
                    table.thiscell.push_str(&format_internal_link(dest_url));
                } else {
                    write!(writer, "{}", format_internal_link(dest_url))?
                }
            }
            Event::Start(Tag::CodeBlock(ck)) => {
                match ck {
                    CodeBlockKind::Fenced(lang) => writeln!(writer, "\n``````{lang}")?,
                    CodeBlockKind::Indented => writeln!(writer, "\n``````")?,
                }
                in_code = true;
            }
            Event::End(TagEnd::Link) => {
                if let Some(table) = &mut table {
                    table.thiscell.push_str("]");
                } else {
                    write!(writer, "]")?;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                writeln!(writer, "\n``````")?
            }
            Event::Start(Tag::List(l)) => {
                writeln!(writer)?;
                list = l;
            }
            Event::Start(Tag::Item) => {
                if let Some(l) = &mut list {
                    write!(writer, "{l}. ")?;
                    *l += 1;
                } else {
                    write!(writer, "- ")?;
                }
                in_listitem = true;
            }
            Event::End(TagEnd::Item) => {
                writeln!(writer)?;
                in_listitem = false;
            }
            Event::End(TagEnd::List(_)) => {
                list = None;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let hl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                } + chap_level;
                if number {
                    write!(
                        writer,
                        "\n{} ",
                        std::iter::repeat("=").take(hl).collect::<String>(),
                    )?;
                } else {
                    write!(writer, "\n*")?;
                }
                in_head = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_head = false;
                if number {
                    writeln!(writer)?;
                } else {
                    write!(writer, ":*\\\n")?;
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                if let Some(path) = chap_path.as_ref().and_then(|p| p.parent()) {
                    write!(
                        writer,
                        "\n#figure(image({:?}), caption: [",
                        path.join(dest_url.to_string())
                    )?
                } else {
                    write!(
                        writer,
                        "\n#figure(image({:?}), caption: [",
                        dest_url.to_string()
                    )?
                }
            }
            Event::End(TagEnd::Image) => {
                writeln!(writer, "])")?;
            }
            Event::Start(Tag::Table(al)) => {
                let mut tab = MdTable::default();
                tab.aligns = al
                    .into_iter()
                    .map(|a| match a {
                        Alignment::None => "none",
                        Alignment::Left => "left",
                        Alignment::Right => "right",
                        Alignment::Center => "center",
                    })
                    .collect();
                table = Some(tab);
            }
            Event::Start(Tag::TableHead) => {
                if let Some(table) = &mut table {
                    table.on_cell = false;
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = &mut table {
                    table.on_cell = true;
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = &mut table {
                    let cell = table.thiscell.clone();
                    table.thiscell.clear();
                    if table.on_cell {
                        table.cells.push(cell);
                    } else {
                        table.headers.push(cell);
                    }
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(table) = table.take() {
                    writeln!(
                        writer,
                        "
#table(
  columns: {},
  table.header({}),
  {}
)
",
                        table.aligns.len(),
                        table
                            .headers
                            .iter()
                            .map(|h| format!("[*{h}*]"))
                            .collect::<Vec<String>>()
                            .join(", "),
                        table
                            .cells
                            .iter()
                            .map(|h| format!("[{h}]"))
                            .collect::<Vec<String>>()
                            .join(", "),
                    )?
                }
            }

            // Event::FootnoteReference(r) => write!(writer, "#ft()")?,
            _ => (),
        }
        consec_par = false;
    }

    Ok(())
}

fn maybe_label(chap_name: &str, text: pulldown_cmark::CowStr) -> String {
    if let Some((pre, post)) = text.split_once(" { #") {
        let label = post.trim().trim_end_matches('}').trim();
        format!(
            "{pre} <{}:{label}>",
            chap_name.to_lowercase().replace(" ", "_")
        )
    } else {
        escape_typst(text)
    }
}

fn format_internal_link(link: pulldown_cmark::CowStr) -> String {
    if link.contains(".md#") {
        if let Some((file, func)) = link.split_once('#') {
            let path = PathBuf::from(file);
            let fname = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .replace(" ", "_");
            return format!("#link(<{fname}:{func}>)[");
        }
    }
    format!("#link(\"{link}\")[")
}

fn escape_typst(text: pulldown_cmark::CowStr) -> String {
    text.replace('*', "\\*").replace('#', "\\#")
}

fn html_block(html: pulldown_cmark::CowStr) -> String {
    // <div class="right">
    if let Some(residue) = html.trim().strip_prefix("<div class=\"") {
        if let Some((name, content)) = residue.split_once("\">") {
            if content.contains("</div>") {
                return format!("#htmlblock({name:?})[{}]", content.replace("</div", ""));
            } else {
                return format!("#htmlblock({name:?})[{content}");
            }
        }
    }

    if html.trim().starts_with("<!--") {
        return String::new();
    }

    match html.trim() {
        "<div class=\"warning\">" => "#htmlblock(\"warning\")[".to_string(),
        "</div>" => "]".to_string(),
        "<center>" => "".to_string(),
        "</center>" => "".to_string(),
        _ => html.to_string(),
    }
}
