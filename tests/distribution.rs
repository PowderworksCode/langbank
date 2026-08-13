//! Tools from mason: what they are, and where they are published.
//!
//! Mason is the inverse index of nvim-lspconfig. lspconfig knows how to run a
//! tool; mason knows what roles it fills and how it is distributed — and it
//! says so in purl, which is the vocabulary `data/registries/` already carries.

use langbank::*;

#[test]
fn distribution_is_expressed_in_the_registry_vocabulary_langbank_already_has() {
    let ruff = toolchain("lsp-ruff").expect("ruff");
    let distribution = ruff.distribution.expect("ruff is distributed");
    assert_eq!(distribution.registry, "github");
    assert_eq!(distribution.package, "astral-sh/ruff");
    // and it resolves to the purl type, not just a string that looks like one
    let registry = distribution
        .package_registry()
        .expect("github is a purl type");
    assert_eq!(registry.id, "github");
}

#[test]
fn a_registry_purl_does_not_define_resolves_to_nothing_rather_than_a_guess() {
    // Mason publishes some packages under `openvsx`, which is not a purl type.
    // The string is kept because it is what mason says; it simply does not
    // resolve, which is different from resolving to something wrong.
    let unknown = toolchains()
        .iter()
        .filter_map(|entry| entry.distribution)
        .filter(|d| d.package_registry().is_none())
        .count();
    assert!(unknown > 0, "openvsx packages exist and do not resolve");
    for entry in toolchains() {
        if let Some(d) = entry.distribution {
            assert!(
                !d.registry.is_empty() && !d.package.is_empty(),
                "{}",
                entry.id
            );
        }
    }
}

#[test]
fn a_tool_can_fill_several_roles_at_once() {
    // ruff is a linter, a formatter and a language server. Collapsing that to
    // one kind loses whichever question the consumer was actually asking.
    let ruff = toolchain("lsp-ruff").expect("ruff");
    assert!(ruff.is(ToolchainKind::Linter));
    assert!(ruff.is(ToolchainKind::Formatter));
    assert!(ruff.is(ToolchainKind::LanguageServer));
    assert!(!ruff.is(ToolchainKind::Debugger));
}

#[test]
fn merging_did_not_disturb_what_lspconfig_already_established() {
    // ruff gained categories and a distribution; its root markers and program
    // came from lspconfig and must be untouched.
    let ruff = toolchain("lsp-ruff").expect("ruff");
    assert_eq!(ruff.programs, &["ruff"]);
    assert!(ruff.root_markers.contains(&"pyproject.toml"));
    // and the hand-written compiler entries kept their probes
    assert!(toolchain("rustc").and_then(|t| t.version).is_some());
}

#[test]
fn tools_mason_alone_knows_about_became_their_own_entries() {
    let shellcheck = toolchain("mason-shellcheck").expect("shellcheck");
    assert!(shellcheck.is(ToolchainKind::Linter));
    assert!(shellcheck.handles(language_profile("shell").expect("shell")));
    assert!(shellcheck.distribution.is_some());
    // no version probe: mason says nothing about how to interrogate a program
    assert!(shellcheck.version.is_none());
}

#[test]
fn debuggers_are_a_role_langbank_did_not_previously_have() {
    let debuggers = toolchains()
        .iter()
        .filter(|entry| entry.is(ToolchainKind::Debugger))
        .count();
    assert!(debuggers > 5, "{debuggers} debug adapters");
}

#[test]
fn a_language_can_be_asked_what_tooling_exists_for_it() {
    let python = language_profile("python").expect("python");
    let tools = toolchains_for(python);
    assert!(tools.len() > 20, "python has {} tools", tools.len());
    for kind in [
        ToolchainKind::Linter,
        ToolchainKind::Formatter,
        ToolchainKind::LanguageServer,
    ] {
        assert!(
            tools.iter().any(|entry| entry.is(kind)),
            "python should have a {kind:?}"
        );
    }
}
