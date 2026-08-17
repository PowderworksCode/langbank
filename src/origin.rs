//! Where a thing lives: its own site, and its source.
//!
//! Two fields, and a rule about when to fill them in. `repository` is where the
//! code is. `homepage` is the project's own site — and it is `None` when that
//! site *is* the repository, which is the common case for a tool that never
//! built itself a page.
//!
//! Recording `https://github.com/eslint/eslint` twice would not be wrong
//! exactly, but it would make a reader look for a difference that is not there,
//! and it would make "has a homepage" a useless thing to ask. Of the 656 tools
//! in static-analysis that publish both, 368 publish the same URL twice.
//!
//! So: one link when a project has only its code, two when it has somewhere
//! else to send you.

/// Where to read about a thing, and where to read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Origin {
    /// The project's own site, when it has one that is not merely its
    /// repository. `None` here and `Some` in `repository` means the code is all
    /// there is.
    pub homepage: Option<&'static str>,
    /// Where the source is kept.
    pub repository: Option<&'static str>,
}

impl Origin {
    pub const UNKNOWN: Origin = Origin {
        homepage: None,
        repository: None,
    };

    /// Neither is recorded — which is a gap rather than a claim that a project
    /// has no website.
    pub fn is_unknown(&self) -> bool {
        self.homepage.is_none() && self.repository.is_none()
    }

    /// Every link worth showing, labelled, in the order a reader wants them:
    /// the project's own page first when it has one, then the code.
    pub fn links(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        [("website", self.homepage), ("source", self.repository)]
            .into_iter()
            .filter_map(|(label, url)| url.map(|url| (label, url)))
    }

    /// The one link to follow if you only follow one.
    pub fn primary(&self) -> Option<&'static str> {
        self.homepage.or(self.repository)
    }
}

/// Normalise a URL for comparison: trailing slashes and case in the host are
/// not differences worth carrying, and `http` versus `https` is not one either
/// when the rest matches.
///
/// Used by the sync tools to decide whether a published homepage is really the
/// repository under another spelling.
pub fn same_place(left: &str, right: &str) -> bool {
    fn key(url: &str) -> String {
        url.trim()
            .trim_end_matches('/')
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.")
            .to_lowercase()
    }
    key(left) == key(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_homepage_that_is_the_repository_is_the_same_place() {
        assert!(same_place(
            "https://github.com/eslint/eslint",
            "https://github.com/eslint/eslint/"
        ));
        assert!(same_place(
            "http://github.com/Foo/Bar",
            "https://github.com/foo/bar"
        ));
        assert!(!same_place(
            "https://abaplint.org",
            "https://github.com/abaplint/abaplint"
        ));
    }

    #[test]
    fn links_are_labelled_and_ordered() {
        let both = Origin {
            homepage: Some("https://biomejs.dev"),
            repository: Some("https://github.com/biomejs/biome"),
        };
        assert_eq!(
            both.links().collect::<Vec<_>>(),
            [
                ("website", "https://biomejs.dev"),
                ("source", "https://github.com/biomejs/biome")
            ]
        );

        // The common case: code is all there is, so there is one link.
        let code_only = Origin {
            homepage: None,
            repository: Some("https://github.com/eslint/eslint"),
        };
        assert_eq!(code_only.links().count(), 1);
        assert_eq!(
            code_only.primary(),
            Some("https://github.com/eslint/eslint")
        );
        assert!(Origin::UNKNOWN.is_unknown());
    }
}
