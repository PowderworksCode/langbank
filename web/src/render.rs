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

use maud::{DOCTYPE, Markup, html};

/// The stylesheet, compiled in rather than read from disk — the site still
/// opens no files at run time.
pub const CSS: &str = include_str!("site.css");

/// The URL the stylesheet is served from, with a hash of its contents in the
/// name.
///
/// It is linked rather than inlined because this site is meant to be clicked
/// through: the CSS is 5 KB and a language page is 11 KB, so inlining it made
/// nearly half of every page a copy of the previous one. Linked and marked
/// immutable, a visitor reading twenty languages fetches it once.
///
/// The hash is what makes `immutable` safe to promise. Change the file and the
/// URL changes, so nobody is holding a stale copy; leave it alone across a
/// deploy and nobody re-downloads it. FNV-1a is enough — this needs to notice a
/// change, not resist an adversary.
pub fn stylesheet_path() -> &'static str {
    static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in CSS.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        format!("/site.{hash:016x}.css")
    })
}

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

/// A proportional bar, as a share of `largest` rather than of a total.
///
/// Against the total, every row but the biggest is a sliver and the shape is
/// invisible — which defeats the point of drawing it.
pub fn bar(part: usize, largest: usize) -> Markup {
    let percent = part
        .checked_mul(100)
        .and_then(|n| n.checked_div(largest))
        .unwrap_or(0);
    html! { span.bartrack { span.bar style=(format!("width:{percent}%")) {} } }
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
        link rel="stylesheet" href=(stylesheet_path());
        header {
            nav {
                a.brand href="/" { "langbank" }
                a href="/languages" { "languages" }
                a href="/ecosystems" { "ecosystems" }
                a href="/toolchains" { "toolchains" }
                a href="/registries" { "registries" }
                a href="/coverage" { "coverage" }
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
