//! The generated registry matches what the hand-written profiles said.
//!
//! `data/` is now the source of truth and `build.rs` turns it into the same
//! statics that used to be written out by hand. These tests are the regression
//! suite for that move: they assert the facts the Rust profiles asserted, so a
//! TOML file that silently loses a field fails here rather than downstream.

use langbank::*;

#[test]
fn every_language_file_produced_a_profile() {
    // A TOML file that fails to parse panics the build; one that parses but
    // loses its id would silently vanish from the registry, so the count is
    // asserted rather than assumed.
    assert_eq!(
        language_profiles().len(),
        29,
        "expected one profile per file in data/languages"
    );
}

#[test]
fn cross_references_resolve_through_the_generator() {
    let typescript = language_profile("typescript").expect("typescript");

    // supersedes names another language by id and must come out as a pointer
    // to that language's profile, not a copy of it.
    let javascript = language_profile("javascript").expect("javascript");
    assert!(
        typescript.supersedes(javascript),
        "typescript supersedes javascript"
    );

    // facets name entries in a different registry
    let facets = typescript.facets.iter().map(|f| f.id).collect::<Vec<_>>();
    assert!(facets.contains(&"structured-code"));
    assert!(facets.contains(&"component-host"));
}

#[test]
fn comment_tables_are_shared_not_copied() {
    // javascript and typescript named the same table in TOML, so they must
    // come out pointing at one static rather than at two equal ones.
    let js = language_profile("javascript")
        .and_then(|p| p.comments)
        .expect("js comments");
    let ts = language_profile("typescript")
        .and_then(|p| p.comments)
        .expect("ts comments");
    assert!(std::ptr::eq(js, ts), "one table, shared");
    assert!(js.line.contains(&"//"));
    assert!(js.multi_quotes.contains(&"`"));

    let rust = language_profile("rust")
        .and_then(|p| p.comments)
        .expect("rust comments");
    assert!(!std::ptr::eq(js, rust), "rust has its own table");
    assert_eq!(rust.quotes, &['"'], "rust has no single-quoted strings");
}

#[test]
fn inline_test_rules_answer_as_the_hand_written_detectors_did() {
    let rust = language_profile("rust")
        .and_then(|profile| profile.conventions)
        .expect("rust conventions");
    assert_eq!(
        rust.inline_test_indicator("#[cfg(test)]\nmod t {}"),
        Some("#[cfg(test)]")
    );
    assert_eq!(
        rust.inline_test_indicator("    #[test]\nfn t() {}"),
        Some("#[test]")
    );
    assert_eq!(
        rust.inline_test_indicator("mod tests {}"),
        Some("mod tests")
    );
    assert_eq!(rust.inline_test_indicator("fn main() {}"), None);
    // leading whitespace is trimmed, and the first matching line wins
    assert_eq!(
        rust.inline_test_indicator("fn main() {}\n\n  #[cfg(test)]\n  mod tests {}"),
        Some("#[cfg(test)]")
    );

    let javascript = language_profile("javascript")
        .and_then(|profile| profile.conventions)
        .expect("javascript conventions");
    assert_eq!(
        javascript.inline_test_indicator("describe('x', () => {})"),
        Some("describe")
    );
    assert_eq!(
        javascript.inline_test_indicator("describe.each([])"),
        Some("describe")
    );
    assert_eq!(
        javascript.inline_test_indicator("it('x', () => {})"),
        Some("it")
    );
    assert_eq!(
        javascript.inline_test_indicator("test.skip('x')"),
        Some("test")
    );
}

#[test]
fn a_contains_any_rule_narrows_a_prefix_that_is_otherwise_ordinary() {
    // `import ` alone must not count — that is the whole reason the rule form
    // needs a second condition rather than just a prefix list.
    let javascript = language_profile("javascript")
        .and_then(|profile| profile.conventions)
        .expect("javascript conventions");
    assert_eq!(
        javascript.inline_test_indicator("import fs from \"node:fs\";"),
        None
    );
    assert_eq!(
        javascript.inline_test_indicator("import { test } from \"vitest\";"),
        Some("test framework import")
    );
    assert_eq!(
        javascript.inline_test_indicator("import { it } from 'vitest';"),
        Some("test framework import")
    );
    assert_eq!(
        javascript.inline_test_indicator("import { describe } from \"@jest/globals\";"),
        Some("test framework import")
    );
    assert_eq!(
        javascript.inline_test_indicator("import { test } from \"node:test\";"),
        Some("test framework import")
    );
}

#[test]
fn typecheck_and_layout_conventions_survive_the_round_trip() {
    let typescript = language_profile("typescript")
        .and_then(|profile| profile.conventions)
        .expect("typescript conventions");
    assert_eq!(
        typescript.typecheck.map(|check| check.config_files),
        Some(&["tsconfig.json"][..])
    );
    assert_eq!(typescript.test_layout.source_roots, &["src"]);
    assert_eq!(typescript.test_layout.test_root, "tests");
    assert!(typescript.test_layout.test_suffixes.contains(&".spec"));

    // a language with no typecheck convention says so rather than inventing one
    let rust = language_profile("rust")
        .and_then(|profile| profile.conventions)
        .expect("rust conventions");
    assert!(rust.typecheck.is_none());
}

#[test]
fn a_language_that_states_no_source_extensions_accepts_what_identifies_it() {
    let rust = language_profile("rust").expect("rust");
    assert_eq!(rust.extensions, rust.source_extensions);
    // typescript states a wider set, and it must not have been collapsed
    let typescript = language_profile("typescript").expect("typescript");
    assert!(typescript.source_extensions.contains(&"js"));
    assert!(!typescript.extensions.contains(&"js"));
}
