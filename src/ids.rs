//! String-newtype identifiers, and the macro that mints them.
//!
//! Every registry in this crate is keyed by one of these. The macro is public
//! because a consumer with its own registries — entl's packages and workspaces,
//! for instance — should spell its identifiers the same way rather than
//! inventing a second convention.

/// Declare a transparent string newtype with the fleet's identifier
/// conventions: ordered, hashable, serialised as a bare string.
#[macro_export]
macro_rules! string_id {
    ($(#[$meta:meta])* $vis:vis $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
            ::serde::Serialize, ::serde::Deserialize,
        )]
        #[serde(transparent)]
        $vis struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

string_id!(
    /// A language, as this crate names it: `rust`, `typescript`, `c`.
    pub LanguageId
);
string_id!(
    /// A build output kind: `binary`, `site`, `napi`.
    pub ArtifactId
);
string_id!(
    /// A package ecosystem: `cargo`, `npm`, `pnpm`.
    pub EcosystemId
);
string_id!(
    /// A reusable source surface a project exposes.
    pub ProjectFacetId
);
