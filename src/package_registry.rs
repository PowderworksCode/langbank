//! Where a package identity lives.
//!
//! A registry is the namespace a package is named in — `pkg:npm/lodash@4` —
//! and it is not the tool that manages the package. npm, pnpm, yarn and bun are
//! four managers over one registry, and they differ in lockfile and workspace
//! selector rather than in what a package is called. Conflating the two makes
//! it impossible to say that a `pnpm-lock.yaml` and a `package-lock.json`
//! describe packages from the same place.
//!
//! Ids follow purl, which is the industry's answer to "what do we call this
//! ecosystem" and is already what SBOM tooling emits.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use crate::registry;

/// Whether a component of a package identity has to be present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    Required,
    Optional,
    Prohibited,
}

/// How one component of a package identity behaves.
#[derive(Debug, Clone, Copy)]
pub struct IdentityComponent {
    pub requirement: Requirement,
    /// Whether `Foo` and `foo` are different packages. They are in npm and are
    /// not in several others, which is exactly the sort of fact a consumer
    /// comparing two identities has no business guessing at.
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PackageRegistry {
    pub id: &'static str,
    pub display_name: &'static str,
    /// The canonical host, where there is one. `generic` and `github` have none.
    pub default_repository: Option<&'static str>,
    pub uses_repository: bool,
    pub namespace: IdentityComponent,
    pub name: IdentityComponent,
    pub version: IdentityComponent,
}

#[derive(Debug, Clone, Copy)]
pub struct PackageRegistryRegistration(pub &'static PackageRegistry);

registry::collect!(PackageRegistryRegistration);

static REGISTERED: LazyLock<Vec<&'static PackageRegistry>> = LazyLock::new(|| {
    let mut registries = registry::iter::<PackageRegistryRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    registries.sort_by_key(|registry| registry.id);
    let mut ids = BTreeSet::new();
    for entry in &registries {
        assert!(ids.insert(entry.id), "duplicate package registry ID");
    }
    registries
});

pub fn package_registries() -> &'static [&'static PackageRegistry] {
    &REGISTERED
}

pub fn package_registry(id: &str) -> Option<&'static PackageRegistry> {
    package_registries()
        .iter()
        .copied()
        .find(|entry| entry.id == id)
}
