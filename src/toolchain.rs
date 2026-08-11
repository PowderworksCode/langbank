//! The programs a language is processed by, and how to interrogate them.
//!
//! This is not [`crate::ToolProfile`], which classifies an invocation — what
//! task does `cargo test` perform, what does it leave behind. A toolchain entry
//! is about the program itself: which one implements a language, how to find
//! out whether it is installed and at what version, and how to ask it for
//! machine-readable diagnostics.
//!
//! Both are needed and they answer different questions. A consumer recording
//! that an observation came from rustc 1.97.1 wants this; a consumer deciding
//! that `cargo test` is a test run wants the other.
//!
//! Nothing here executes anything. Langbank supplies the arguments, the stream
//! to read and the pattern to apply, and the consumer runs the program —
//! which is why `pattern` is a string rather than a compiled regex, and why
//! this crate has no regex dependency to force on the fleet.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use crate::{LanguageProfile, registry};

/// What a program does for a language. Coarse on purpose: a program often does
/// several of these and the distinction that matters is what a consumer is
/// looking for, not a taxonomy of software.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolchainKind {
    Compiler,
    Runtime,
    PackageManager,
    BuildSystem,
    TypeChecker,
    Formatter,
    Linter,
    LanguageServer,
}

/// Which stream a program prints to. Not decoration: `java -version` writes to
/// stderr while `javac -version` writes to stdout, same vendor and same flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// How to read a program's version.
#[derive(Debug, Clone, Copy)]
pub struct VersionProbe {
    pub arguments: &'static [&'static str],
    pub stream: OutputStream,
    /// Applied to the first non-empty line; capture group 1 is the version.
    /// A string rather than a compiled regex, because compiling one is the
    /// consumer's job and a regex crate is not a dependency this leaf imposes.
    pub pattern: &'static str,
}

/// How to ask a program for diagnostics a machine can read.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticFormat {
    /// `json`, and whatever else turns up. A string rather than an enum: the
    /// set grows with the tools, and an enum would churn the schema each time.
    pub format: &'static str,
    pub arguments: &'static [&'static str],
    pub stream: OutputStream,
}

#[derive(Debug, Clone, Copy)]
pub struct Toolchain {
    pub id: &'static str,
    pub display_name: &'static str,
    pub kind: ToolchainKind,
    pub languages: &'static [&'static LanguageProfile],
    /// In preference order. The unversioned name is frequently absent where the
    /// program is installed — packaged clang lands as `clang-21` — so a
    /// consumer that probes only the first entry will decide a machine has no C
    /// compiler when it has two.
    pub programs: &'static [&'static str],
    pub version: Option<VersionProbe>,
    pub diagnostics: Option<DiagnosticFormat>,
}

impl Toolchain {
    pub fn handles(&self, language: &LanguageProfile) -> bool {
        self.languages
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, language))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToolchainRegistration(pub &'static Toolchain);

registry::collect!(ToolchainRegistration);

static REGISTERED: LazyLock<Vec<&'static Toolchain>> = LazyLock::new(|| {
    let mut toolchains = registry::iter::<ToolchainRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    toolchains.sort_by_key(|toolchain| toolchain.id);
    let mut ids = BTreeSet::new();
    for toolchain in &toolchains {
        assert!(ids.insert(toolchain.id), "duplicate toolchain ID");
    }
    toolchains
});

pub fn toolchains() -> &'static [&'static Toolchain] {
    &REGISTERED
}

pub fn toolchain(id: &str) -> Option<&'static Toolchain> {
    toolchains().iter().copied().find(|entry| entry.id == id)
}

/// Every toolchain that processes a language, in registry order.
pub fn toolchains_for(language: &LanguageProfile) -> Vec<&'static Toolchain> {
    toolchains()
        .iter()
        .copied()
        .filter(|toolchain| toolchain.handles(language))
        .collect()
}
