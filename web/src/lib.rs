//! The site's pages, as functions from the registry to HTML.
//!
//! Kept apart from the binary so the tests can render a page without binding a
//! port. Nothing here touches the network, the filesystem or the clock: a page
//! is a pure function of langbank's compiled-in data and, for `identify`, the
//! two strings a visitor supplied.

mod pages;
pub mod render;

pub use pages::identify::Query;

pub fn home() -> String {
    pages::home::render()
}

pub fn languages() -> String {
    pages::languages::index()
}

/// `None` when no such language, so the caller can answer 404 rather than
/// render a page about nothing.
pub fn language(id: &str) -> Option<String> {
    pages::languages::detail(id)
}

pub fn identify(path: &str, content: &str) -> String {
    pages::identify::render(&Query {
        path: path.to_string(),
        content: content.to_string(),
    })
}

pub fn identify_query(query: &Query) -> String {
    pages::identify::render(query)
}

pub fn ecosystems() -> String {
    pages::tables::ecosystems()
}

pub fn toolchains() -> String {
    pages::tables::toolchains()
}

pub fn registries() -> String {
    pages::tables::registries()
}

pub fn tools() -> String {
    pages::tables::tools()
}

pub fn gaps() -> String {
    pages::tables::gaps()
}

/// The 404 body, which names what was asked for: a visitor who mistyped a
/// language id should see that rather than have to guess.
pub fn not_found(subject: &str) -> String {
    render::page(
        "Not found",
        &[("/", "langbank")],
        &format!(
            "<h1>Not found</h1><p class=lede>langbank carries no {}.</p>\
             <p>{} lists everything it does.</p>",
            render::code(subject),
            render::link("/languages", "The language index")
        ),
    )
}
