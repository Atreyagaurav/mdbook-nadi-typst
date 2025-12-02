# Mdbook renderer for Typst

The renderer generates a typst document you can copy to project root and compile.

The possible configuration option includes a prelude file.

```toml
[output.typst]
prelude = "prelude.typ"
```

Or you can use the `prelude-str` key to directly put the prelude string there.

If not included it will use the default prelude:

```typst
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
```


While writing your own prelude make sure you have the `unum_chap`, `bookpart` and `htmlblock` functions defined. They are used to format the unnumbered chapters, book parts and html blocks in the mdbook as typst does not have syntax for those.

This is an experimental renderer I wrote to export mdbook for my personal use, if there are issues and you like it to be fixed, please make an issue on github I will try to make sure it can be used for various use cases.

But as typst itself is yet not stable, and the previous attempts of mdbook-typst has been deprecated, I can not guarantee the same won't happen to this project.
