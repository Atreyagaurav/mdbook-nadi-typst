use mdbook::book::{BookItem, Chapter};
use mdbook::renderer::RenderContext;
use pulldown_cmark::{Options, Parser};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let mut stdin = std::io::stdin();
    let ctx = RenderContext::from_json(&mut stdin).unwrap();

    let _ = std::fs::create_dir_all(&ctx.destination);
    let book_path = ctx.destination.join("book.typ");

    let file = std::fs::File::create(book_path)?;
    let mut writer = std::io::BufWriter::new(file);
    writeln!(
        writer,
        "{}",
        r#"
#set document(title: "Network Analysis and Data Integration (NADI) Book")
#set document(author: ("Gaurav Atreya"))
#set heading(numbering: "1.", depth: 3)
#set page(paper: "us-letter")
#set text(size: 11pt)
#set text(font: "Noto Sans")
#set par(spacing:2em, leading: .8em, justify: true)
#set raw(syntaxes: "typst/task.sublime-syntax")
#set raw(syntaxes: "typst/signature.sublime-syntax")
#set raw(syntaxes: "typst/stp.sublime-syntax")

#show heading: it => [
    #block(above: 2em, below: 2em, it)
]
#show link: it => {
  if type(it.dest) != str {
    underline(text(fill:green, it))
  }
  else {
    underline(text(fill:blue, it))
  }
}
#show ref: underline
#show ref: set text(green)
#show raw: set block(fill: luma(230), inset: 8pt, radius: 4pt, width: 100%)
#show outline.entry.where(level: 1):it => {
                                    v(11pt, weak: true)
                                   strong(it) 
                                }

#{
    set page(fill: gradient.linear(luma(100), luma(200)).sharp(20, smoothness: 40%))
    set align(center)
    text(17pt, [Network Analysis and Data Integration (NADI) System])
    v(2mm)
    text(17pt, [User Manual])

    {
        image("cover.png", width:100%)
    }
    text(27pt, [NADI Book ])
    text(17pt, [Version: 0.7.0])

    v(1mm)
    text(12pt, [Web Version: ] + link("https://nadi-system.github.io/"))

    v(1fr)
    grid(
        rows: 0.5cm,
        columns: 1,
        [Gaurav Atreya],
        [2025-06-27]
    )
}

#pagebreak()
#set page(numbering: "i")
#counter(page).update(1)

#let unum_chap(contents) = align(center, text(size:16pt, contents))
#let bookpart(contents) = block(fill:luma(200), inset: 8pt, width: 100%, align(center, text(size:16pt, contents)))

#show quote: set block(fill: luma(230), inset: 8pt, radius: 4pt, width: 100%)
#let htmlblock(cat, contents) = block(fill: yellow, inset: 8pt, radius: 4pt, width: 100%, contents)

#outline(depth: 2, indent: 2em)
#pagebreak()
#counter(page).update(1)
#set page(numbering: "1", header:[#h(1fr) Nadi Book])
"#
    )?;

    for section in ctx.book.sections {
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
    write_markdown(writer, contents, level, chapter.path, number)?;

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
    number: bool,
) -> std::io::Result<()> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(&md, options);
    use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};

    let mut table: Option<MdTable> = None;
    let mut list: Option<u64> = None;
    let mut in_code = false;
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
                    c.lines()
                        .map(|l| l.trim_start_matches('!'))
                        .collect::<Vec<&str>>()
                        .join("\n")
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
            Event::Start(Tag::Paragraph) => write!(writer, "\n\n")?,
            Event::End(TagEnd::Paragraph) => write!(writer, "\n\n")?,
            Event::Start(Tag::Strong) => write!(writer, "*")?,
            Event::End(TagEnd::Strong) => write!(writer, "*")?,
            Event::Start(Tag::Link { dest_url, .. }) => write!(writer, "#link(\"{dest_url}\")[")?,
            Event::Start(Tag::CodeBlock(ck)) => {
                match ck {
                    CodeBlockKind::Fenced(lang) => writeln!(writer, "\n``````{lang}")?,
                    CodeBlockKind::Indented => writeln!(writer, "\n``````")?,
                }
                in_code = true;
            }
            Event::End(TagEnd::Link) => write!(writer, "]")?,
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
            }
            Event::End(TagEnd::Item) => {
                writeln!(writer)?;
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
            }
            Event::End(TagEnd::Heading(_)) => {
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
                    write!(writer, "\n#figure(image(\"{dest_url}\"), caption: [")?
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
    }

    Ok(())
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
