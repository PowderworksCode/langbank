//! Language, ecosystem and toolchain data for the fleet.
//!
//! What a language is, how to recognise it, what conventions it carries, which
//! ecosystem publishes it, and what its tooling can be asked. Everything here
//! is a registry over static data plus the few functions needed to look
//! something up; nothing here walks a filesystem, spawns a process, or parses
//! a source file.
//!
//! It sits at the bottom of the fleet deliberately. Entl names languages while
//! walking a tree, treebank names them when it publishes a grammar, and
//! propbank names them when it observes a program. All three need the same
//! vocabulary, and none of them should have to depend on another to get it.
//!
//! Registries are separate on purpose. An ecosystem role is not a language, a
//! language is not a toolchain, and an artifact is not either.

mod artifact;
mod convention;
mod dependency;
mod detection;
mod ecosystem;
mod facet;
mod ids;
mod language;
mod tool;
mod traversal;
mod verbosity;

// The registries are generated from `data/` at build time.
include!(concat!(env!("OUT_DIR"), "/registries.rs"));

pub use artifact::{ArtifactProfile, ArtifactRegistration, artifact_profile, artifact_profiles};
pub use artifacts::{BINARY_ARTIFACT, NAPI_ARTIFACT, SITE_ARTIFACT, TAURI_ARTIFACT};
pub use convention::{
    InlineTestRule, LanguageConventions, TestLayoutDefaults, TypecheckConvention,
    language_conventions,
};
pub use dependency::DependencySource;
pub use detection::{LanguageDetection, LanguageEvidence};
pub use ecosystem::{
    DependencyPinPolicy, DependencyPinStatus, DependencyPinSyntax, EcosystemProfile,
    EcosystemRegistration, EcosystemRole, ManifestSelection, ecosystem_profile, ecosystem_profiles,
};
pub use ecosystems::{
    BUN as BUN_ECOSYSTEM, CARGO as CARGO_ECOSYSTEM, NPM as NPM_ECOSYSTEM, PNPM as PNPM_ECOSYSTEM,
    YARN as YARN_ECOSYSTEM,
};
pub use facet::{LanguageFacet, LanguageFacetRegistration, language_facet, language_facets};
pub use facets::{COMPONENT_HOST, STRUCTURED_CODE, STYLE_HOST};
pub use ids::{ArtifactId, EcosystemId, LanguageId, ProjectFacetId};
pub use language::{
    CommentSyntax, LanguageProfile, LanguageProvenance, LanguageRegistration, LanguageRole,
    comment_syntax, comment_syntax_for_extension, detect_language, language_profile,
    language_profile_for_extension, language_profiles, languages_claiming_extension,
};
pub use languages::css::PROFILE as CSS_LANGUAGE;
pub use languages::javascript::PROFILE as JAVASCRIPT_LANGUAGE;
pub use languages::less::PROFILE as LESS_LANGUAGE;
pub use languages::rust::PROFILE as RUST_LANGUAGE;
pub use languages::scss::PROFILE as SCSS_LANGUAGE;
pub use languages::shell::PROFILE as SHELL_LANGUAGE;
pub use languages::typescript::PROFILE as TYPESCRIPT_LANGUAGE;
pub use tool::{
    ArgumentPattern, CiWorkload, CommandPattern, TaskKind, TestRetryConfiguration,
    TestRetryProfile, TestRetrySignal, ToolId, ToolProfile, ToolRegistration, classify_tool,
    normalize_invocation, tool_profile, tool_profiles,
};
pub use tools::{CODESPELL, STYLELINT, VALE};
pub use traversal::{TraversalDirectory, TraversalDirectoryRegistration, traversal_directories};
pub use verbosity::{
    LanguageVerbosity, VERBOSITY_BASELINE, VERBOSITY_CORPUS, VERBOSITY_CORPUS_REVISION,
    VerbosityRatio, verbosity, verbosity_ratio, verbosity_ratios,
};

pub use registry_inventory as registry;
