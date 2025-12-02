use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename = "kebab-case")]
pub struct Config {
    pub prelude: Option<PathBuf>,
    pub prelude_str: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            prelude: None,
            prelude_str: None,
        }
    }
}

impl Config {
    pub fn prelude(&self, root: &Path) -> std::io::Result<String> {
        if let Some(p) = &self.prelude_str {
            return Ok(p.to_string());
        }
        if let Some(p) = &self.prelude {
            return std::fs::read_to_string(root.join(p));
        }
        // default style if none is given
        Ok(
            r#"
#set heading(numbering: "1.", depth: 3)
#set page(paper: "us-letter")
#set text(size: 11pt)
#set par(spacing:2em, leading: .8em, justify: true)
#show raw: set block(fill: luma(230), inset: 8pt, radius: 4pt, width: 100%)

#let unum_chap(contents) = align(center, text(size:16pt, contents))
#let bookpart(contents) = block(fill:luma(200), inset: 8pt, width: 100%, align(center, text(size:16pt, contents)))

#show quote: set block(fill: luma(230), inset: 8pt, radius: 4pt, width: 100%)
#let htmlblock(cat, contents) = block(fill: yellow.lighten(50%), inset: 8pt, radius: 4pt, width: 100%, contents)

#set page(numbering: "i")
#counter(page).update(1)
#outline(depth: 2, indent: 2em)
#pagebreak()
#counter(page).update(1)
#set page(numbering: "1")
"#.to_string(),
        )
    }
}
