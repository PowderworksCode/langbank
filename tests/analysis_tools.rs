//! Linters and formatters from analysis-tools-dev/static-analysis.
//!
//! This is the source that answers the question langbank's README opens with —
//! what tooling can this language be asked through — for languages nobody was
//! going to hand-model. It is largely disjoint from mason: mason indexes what
//! an editor can install, this indexes what an analyser community has written,
//! and only 88 of 755 names were already known.

use langbank::*;

#[test]
fn the_analysis_corpus_landed() {
    let linters = toolchains()
        .iter()
        .filter(|entry| entry.is(ToolchainKind::Linter))
        .count();
    assert!(linters > 400, "{linters} linters");
}

#[test]
fn a_language_can_be_asked_what_will_analyse_it() {
    // The point of the whole registry, for languages that were previously a
    // name and an extension and nothing else.
    for (id, least) in [
        ("python", 20),
        ("java", 20),
        ("php", 15),
        ("c", 15),
        ("ruby", 10),
    ] {
        let language = language_profile(id).unwrap_or_else(|| panic!("{id}"));
        let analysers = toolchains_for(language)
            .iter()
            .filter(|entry| entry.is(ToolchainKind::Linter) || entry.is(ToolchainKind::Formatter))
            .count();
        assert!(
            analysers >= least,
            "{id} has {analysers} analysers, wanted {least}"
        );
    }
}

#[test]
fn tools_carry_the_roles_they_actually_fill() {
    let entry = toolchain("sa-shellcheck")
        .or_else(|| toolchain("mason-shellcheck"))
        .expect("shellcheck");
    assert!(entry.is(ToolchainKind::Linter));
    assert!(entry.handles(language_profile("shell").expect("shell")));
}

#[test]
fn analysis_tools_carry_no_probe_or_distribution_they_never_had() {
    // Absence recorded honestly: this source says nothing about how to run a
    // program or where it is published, so nothing is invented for it.
    let invented = toolchains()
        .iter()
        .filter(|entry| entry.id.starts_with("sa-"))
        .filter(|entry| entry.version.is_some() || entry.distribution.is_some())
        .count();
    assert_eq!(
        invented, 0,
        "{invented} entries gained facts this source lacks"
    );
}

#[test]
fn collections_and_benchmarks_were_not_absorbed_as_analysers() {
    // Upstream categorises some entries `meta` (lists of other tools) and one
    // `performance`. Neither is something a language can be asked through.
    for entry in toolchains().iter().filter(|e| e.id.starts_with("sa-")) {
        assert!(
            entry.is(ToolchainKind::Linter) || entry.is(ToolchainKind::Formatter),
            "{} is neither linter nor formatter",
            entry.id
        );
        assert!(!entry.languages.is_empty(), "{} analyses nothing", entry.id);
    }
}

#[test]
fn merging_left_the_existing_entries_alone() {
    // Three tools were already known and gained categories only.
    let ruff = toolchain("lsp-ruff").expect("ruff");
    assert_eq!(ruff.programs, &["ruff"]);
    assert!(
        ruff.distribution.is_some(),
        "ruff kept mason's distribution"
    );
    assert!(toolchain("rustc").and_then(|t| t.version).is_some());
    assert!(toolchain("gcc").and_then(|t| t.diagnostics).is_some());
}
