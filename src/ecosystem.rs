use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::{DependencySource, EcosystemId, LanguageProfile, language_profiles};

use crate::registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EcosystemRole {
    PackageManager,
    BuildSystem,
    Runtime,
    Toolchain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestSelection {
    Default,
    Lockfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyPinSyntax {
    ExactSemver,
    CargoExactRequirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyPinStatus {
    Pinned,
    Floating,
    Local,
}

#[derive(Debug, Clone, Copy)]
pub struct DependencyPinPolicy {
    pub syntax: DependencyPinSyntax,
    pub advisory: bool,
}

impl DependencyPinPolicy {
    /// Whether a declaration is pinned, floating, or local.
    ///
    /// Takes the source and the requirement rather than a parsed dependency:
    /// the record belongs to whoever read the manifest, and a policy that
    /// borrowed it would drag manifest parsing into this crate.
    pub fn classify(
        self,
        source: DependencySource,
        requirement: Option<&str>,
    ) -> DependencyPinStatus {
        match source {
            DependencySource::LocalPath | DependencySource::Workspace => DependencyPinStatus::Local,
            DependencySource::Git => {
                if requirement.is_some_and(commit_sha) {
                    DependencyPinStatus::Pinned
                } else {
                    DependencyPinStatus::Floating
                }
            }
            DependencySource::Registry => {
                let requirement = requirement.unwrap_or_default().trim();
                let pinned = match self.syntax {
                    DependencyPinSyntax::ExactSemver => exact_semver(requirement),
                    DependencyPinSyntax::CargoExactRequirement => requirement
                        .strip_prefix('=')
                        .is_some_and(|version| !version.trim().is_empty()),
                };
                if pinned {
                    DependencyPinStatus::Pinned
                } else {
                    DependencyPinStatus::Floating
                }
            }
            DependencySource::Unknown => DependencyPinStatus::Floating,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EcosystemProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    pub roles: &'static [EcosystemRole],
    pub implied_languages: &'static [&'static LanguageProfile],
    pub manifest: Option<&'static str>,
    pub lockfiles: &'static [&'static str],
    /// Files whose presence identifies this ecosystem where its manifest does
    /// not settle the question. Two shapes, one meaning: `pnpm-workspace.yaml`
    /// tells pnpm apart from three other managers reading the same
    /// `package.json`, and `build.zig` identifies a Zig project whose
    /// `build.zig.zon` is optional and frequently absent — Bun has no manifest
    /// and is still the largest Zig codebase there is.
    /// Where the project lives and where its code is. `homepage` is set
    /// only when it is somewhere other than the repository.
    pub origin: crate::Origin,
    pub selector_files: &'static [&'static str],
    /// Other filenames that are *also* this ecosystem's manifest. Not a
    /// disambiguator — `build.gradle.kts` does not decide ownership, it is
    /// simply the other way to spell `build.gradle`.
    pub alternate_manifests: &'static [&'static str],
    pub gitignore_patterns: &'static [&'static str],
    pub manifest_selection: ManifestSelection,
    pub dependency_pins: Option<DependencyPinPolicy>,
    /// The registry this manager's packages are named in. npm, pnpm, yarn and
    /// bun all point at the same one; what differs between them is the
    /// lockfile, not the package identity.
    pub registry: Option<&'static crate::PackageRegistry>,
}

impl EcosystemProfile {
    pub fn implies_language(&self, language: &LanguageProfile) -> bool {
        self.implied_languages
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, language))
    }

    pub fn has_role(&self, role: EcosystemRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn lockfile_present(&self, directory: &Path) -> bool {
        self.lockfiles
            .iter()
            .any(|lockfile| directory.join(lockfile).is_file())
    }

    pub fn lockfile_description(&self) -> String {
        self.lockfiles.join(" or ")
    }
}

fn exact_semver(requirement: &str) -> bool {
    let version = requirement.strip_prefix('v').unwrap_or(requirement);
    if version.is_empty()
        || version.bytes().any(|byte| {
            matches!(
                byte,
                b'^' | b'~' | b'<' | b'>' | b'=' | b'*' | b'x' | b'X' | b' ' | b',' | b'|'
            )
        })
    {
        return false;
    }
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let components = core.split('.').collect::<Vec<_>>();
    components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl From<&EcosystemProfile> for EcosystemId {
    fn from(profile: &EcosystemProfile) -> Self {
        Self::new(profile.id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EcosystemRegistration(pub &'static EcosystemProfile);

registry::collect!(EcosystemRegistration);

static REGISTERED: LazyLock<Vec<&'static EcosystemProfile>> = LazyLock::new(|| {
    let mut profiles = registry::iter::<EcosystemRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.id);
    for pair in profiles.windows(2) {
        assert_ne!(pair[0].id, pair[1].id, "duplicate ecosystem profile ID");
    }
    for profile in &profiles {
        for language in profile.implied_languages {
            assert!(
                language_profiles()
                    .iter()
                    .any(|registered| std::ptr::eq(*registered, *language)),
                "ecosystem profile {:?} implies unregistered language {:?}",
                profile.id,
                language.id
            );
        }
    }
    let manifests = profiles
        .iter()
        .filter_map(|profile| profile.manifest)
        .collect::<BTreeSet<_>>();
    for manifest in manifests {
        assert_eq!(
            profiles
                .iter()
                .filter(|profile| {
                    profile.manifest == Some(manifest)
                        && matches!(profile.manifest_selection, ManifestSelection::Default)
                })
                .count(),
            1,
            "manifest {manifest:?} needs exactly one default ecosystem"
        );
    }
    profiles
});

pub fn ecosystem_profiles() -> &'static [&'static EcosystemProfile] {
    REGISTERED.as_slice()
}

pub fn ecosystem_profile(id: &str) -> Option<&'static EcosystemProfile> {
    ecosystem_profiles()
        .binary_search_by_key(&id, |profile| profile.id)
        .ok()
        .map(|index| ecosystem_profiles()[index])
}
