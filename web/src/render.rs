//! The page shell, and the small pieces every page shares.
//!
//! Markup is `maud`: an HTML macro checked at compile time, so an unclosed tag
//! is a build error rather than a page that renders sideways. It also escapes
//! every interpolated value by default, and opting out has to be spelled
//! `PreEscaped`. That matters more here than anywhere else on the site —
//! `/identify` renders a filename and a file body a visitor supplied, and
//! langbank's own data carries `<`, `>` and `&` in shebangs, comment markers
//! and regex patterns. The previous version escaped by hand, which worked, but
//! it worked because every call site remembered to.

use maud::{DOCTYPE, Markup, PreEscaped, html};

/// A `<code>` span.
pub fn code(text: &str) -> Markup {
    html! { code { (text) } }
}

/// A run of `<code>` spans, or an em dash when there are none — an empty cell
/// and a cell that says "nothing here" read differently in a table.
pub fn codes<S: AsRef<str>>(values: &[S]) -> Markup {
    if values.is_empty() {
        return html! { span.none { "—" } };
    }
    html! {
        @for (index, value) in values.iter().enumerate() {
            @if index > 0 { " " }
            (code(value.as_ref()))
        }
    }
}

pub fn link(href: &str, text: &str) -> Markup {
    html! { a href=(href) { (text) } }
}

/// A definition row, skipped when there is nothing to say. Rendering forty
/// empty rows for a language langbank knows three things about would bury the
/// three.
pub fn row(label: &str, value: Option<Markup>) -> Markup {
    html! {
        @if let Some(value) = value {
            div.row { dt { (label) } dd { (value) } }
        }
    }
}

/// The shell every page shares. `title` is the browser tab; the `h1` is the
/// page's own, because they are not always the same thing.
pub fn page(title: &str, breadcrumb: &[(&str, &str)], body: Markup) -> String {
    html! {
        (DOCTYPE)
        html lang="en";
        meta charset="utf-8";
        meta name="viewport" content="width=device-width,initial-scale=1";
        title { (title) " — langbank" }
        meta name="description" content="Structured data about programming languages, ecosystems and toolchains, compiled into a Rust crate with no runtime parsing.";
        style { (PreEscaped(CSS)) }
        header {
            nav {
                a.brand href="/" { "langbank" }
                a href="/languages" { "languages" }
                a href="/ecosystems" { "ecosystems" }
                a href="/toolchains" { "toolchains" }
                a href="/registries" { "registries" }
                a href="/identify" { "identify" }
                a.out href="https://github.com/PowderworksCode/langbank" { "source" }
            }
        }
        main {
            @if !breadcrumb.is_empty() {
                p.crumbs {
                    @for (href, text) in breadcrumb {
                        (link(href, text)) " " span.sep { "/" } " "
                    }
                }
            }
            (body)
        }
        footer {
            p {
                "langbank is a leaf: it depends on nothing and describes what other tools
                 need to agree on. Data is compiled to " code { "&'static" } " at build time —
                 this page parses nothing to render."
            }
            p.quiet {
                "Assembled from " a href="https://github.com/github-linguist/linguist" { "linguist" }
                ", " a href="https://github.com/package-url/purl-spec" { "purl-spec" }
                " and other permissively licensed sources. Every figure on this site is
                 generated from the data in the repository."
            }
        }
    }
    .into_string()
}

const CSS: &str = r#"
:root {
  --bg: #fbfbf9; --fg: #1a1a18; --dim: #6a6a64; --line: #e2e1dc;
  --accent: #7a4a1e; --card: #fff; --mark: #fdf3e3;
  --mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme=light]) {
    --bg: #16161a; --fg: #e8e8e4; --dim: #9a9a92; --line: #2e2e34;
    --accent: #e0a366; --card: #1d1d22; --mark: #2a2318;
  }
}
* { box-sizing: border-box }
body {
  margin: 0; background: var(--bg); color: var(--fg);
  font: 16px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", Inter, system-ui, sans-serif;
  -webkit-text-size-adjust: 100%;
}
header { border-bottom: 1px solid var(--line); background: var(--card) }
nav {
  max-width: 62rem; margin: 0 auto; padding: .75rem 1.25rem;
  display: flex; gap: 1.1rem; align-items: baseline; flex-wrap: wrap;
}
nav a { color: var(--dim); text-decoration: none; font-size: .93rem }
nav a:hover { color: var(--accent) }
nav .brand { font-weight: 700; font-size: 1.05rem; color: var(--fg); letter-spacing: -.02em }
nav .out { margin-left: auto }
main { max-width: 62rem; margin: 0 auto; padding: 1.5rem 1.25rem 4rem }
footer {
  max-width: 62rem; margin: 0 auto; padding: 2rem 1.25rem 3rem;
  border-top: 1px solid var(--line); color: var(--dim); font-size: .87rem;
}
footer p { margin: .5rem 0 }
.quiet { font-size: .82rem; opacity: .8 }
a { color: var(--accent) }
h1 { font-size: 1.9rem; line-height: 1.2; letter-spacing: -.025em; margin: .2rem 0 .5rem }
h2 { font-size: 1.15rem; letter-spacing: -.015em; margin: 2.2rem 0 .6rem }
h3 { font-size: .95rem; margin: 1.4rem 0 .4rem }
.crumbs { color: var(--dim); font-size: .85rem; margin: 0 0 .8rem }
.sep { opacity: .5 }
.lede { font-size: 1.12rem; color: var(--dim); max-width: 44rem; margin: 0 0 1.4rem }
code {
  font-family: var(--mono); font-size: .86em; background: var(--mark);
  padding: .08em .34em; border-radius: 3px; word-break: break-word;
}
pre { background: var(--card); border: 1px solid var(--line); border-radius: 6px;
      padding: .9rem 1rem; overflow-x: auto; font-size: .85rem }
pre code { background: none; padding: 0; font-size: 1em }
.none { color: var(--dim) }
.stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(8.5rem, 1fr));
         gap: .7rem; margin: 1.5rem 0 }
.stat { background: var(--card); border: 1px solid var(--line); border-radius: 7px; padding: .8rem .9rem }
.stat b { display: block; font-size: 1.55rem; letter-spacing: -.03em; line-height: 1.1 }
.stat span { color: var(--dim); font-size: .8rem }
.stat a { text-decoration: none; color: inherit }
dl { margin: 1rem 0 }
.row { display: grid; grid-template-columns: 11rem 1fr; gap: .5rem 1rem;
       padding: .5rem 0; border-top: 1px solid var(--line) }
.row dt { color: var(--dim); font-size: .88rem }
.row dd { margin: 0; min-width: 0 }
@media (max-width: 40rem) { .row { grid-template-columns: 1fr } .row dt { font-size: .8rem } }
table { border-collapse: collapse; width: 100%; font-size: .9rem }
.scroll { overflow-x: auto; margin: 1rem 0 }
th, td { text-align: left; padding: .45rem .7rem; border-bottom: 1px solid var(--line);
         vertical-align: top }
th { color: var(--dim); font-weight: 600; font-size: .82rem; white-space: nowrap }
tbody tr:hover { background: var(--card) }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr));
        gap: .35rem .9rem; margin: 1rem 0; padding: 0; list-style: none }
.grid a { text-decoration: none }
.grid a:hover { text-decoration: underline }
.grid small { color: var(--dim); font-family: var(--mono); font-size: .76rem }
form { background: var(--card); border: 1px solid var(--line); border-radius: 7px;
       padding: 1rem; margin: 1.2rem 0 }
label { display: block; font-size: .85rem; color: var(--dim); margin: .6rem 0 .25rem }
input, textarea {
  width: 100%; font-family: var(--mono); font-size: .85rem; padding: .5rem .6rem;
  border: 1px solid var(--line); border-radius: 5px; background: var(--bg); color: var(--fg);
}
textarea { min-height: 9rem; resize: vertical }
button { margin-top: .9rem; padding: .5rem 1.1rem; font-size: .9rem; font-weight: 600;
         border: 1px solid var(--accent); background: var(--accent); color: var(--card);
         border-radius: 5px; cursor: pointer }
button:hover { opacity: .9 }
.verdict { border: 1px solid var(--line); border-left: 3px solid var(--accent);
           background: var(--card); border-radius: 6px; padding: .9rem 1.1rem; margin: 1.2rem 0 }
.verdict h3 { margin: 0 0 .3rem; font-size: 1.15rem }
.verdict p { margin: .3rem 0; font-size: .9rem; color: var(--dim) }
.tag { display: inline-block; font-size: .74rem; font-family: var(--mono);
       border: 1px solid var(--line); border-radius: 3px; padding: .05em .4em;
       color: var(--dim); margin-right: .3rem }
.bar { display: inline-block; height: .55rem; background: var(--accent);
       border-radius: 2px; vertical-align: middle; min-width: 1px }
.bartrack { display: inline-block; width: 7rem; background: var(--line);
            border-radius: 2px; margin-right: .5rem }
"#;
