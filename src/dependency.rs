//! Where a declared dependency comes from.
//!
//! This is the taxonomy, not the record. A parsed dependency — its name,
//! rename, kind and requirement — belongs to whoever read the manifest.
//! What belongs here is the closed set of places a dependency can come from,
//! because a pin policy is stated in terms of it and pin policies are
//! ecosystem data.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencySource {
    Registry,
    Git,
    LocalPath,
    Workspace,
    Unknown,
}
