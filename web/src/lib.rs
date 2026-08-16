//! The site's pages, as functions from the registry to HTML.
//!
//! Kept apart from the binary so the tests can render a page without binding a
//! port. Nothing here touches the network, the filesystem or the clock: a page
//! is a pure function of langbank's compiled-in data and, for `identify`, the
//! two strings a visitor supplied.

mod pages;
pub mod render;

pub use pages::identify::Query;
pub use render::{CSS, stylesheet_path};

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
        maud::html! {
            h1 { "Not found" }
            p.lede { "langbank carries no " (render::code(subject)) "." }
            p { (render::link("/languages", "The language index")) " lists everything it does." }
        },
    )
}

/// Unset means 8080. Set to something unusable does not.
///
/// `PORT.ok().and_then(|p| p.parse().ok()).unwrap_or(8080)` folds those two
/// cases together, so a typo in `fly.toml` binds 8080 while Fly routes to the
/// port that was meant, and the only symptom is a health check that times out
/// with nothing in the logs to say why. The whole job of that check is to
/// report what is wrong, so a misconfiguration is reported here instead of
/// quietly becoming a different, working-looking configuration.
///
/// Takes the looked-up value rather than reading the environment itself, so it
/// can be tested without mutating process-global state.
pub fn port_from(value: Result<&str, &std::env::VarError>) -> Result<u16, String> {
    match value {
        Err(std::env::VarError::NotPresent) => Ok(8080),
        Err(error) => Err(format!("PORT is set but unreadable: {error}")),
        Ok(value) => value
            .parse()
            .map_err(|error| format!("PORT is set to {value:?}, which is not a port: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::port_from;
    use std::env::VarError;

    #[test]
    fn an_unset_port_is_the_default_and_a_broken_one_is_an_error() {
        assert_eq!(port_from(Err(&VarError::NotPresent)), Ok(8080));
        assert_eq!(port_from(Ok("8090")), Ok(8090));

        // The cases that used to be indistinguishable from "unset".
        for broken in ["8o80", "99999", "", "8080 "] {
            assert!(
                port_from(Ok(broken)).is_err(),
                "{broken:?} silently became a working default"
            );
        }
    }
}
