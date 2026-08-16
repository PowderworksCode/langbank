//! The rules langbank carries, actually run.
//!
//! Until this crate existed, 317 disambiguation rules and every shebang table
//! were data asserted to work and never executed. These are the first tests
//! anywhere in the project that check the registry is *correct* rather than
//! merely populated.

use langbank_detect::{Evidence, Undecided, identify};

#[test]
fn a_name_is_enough_when_only_one_language_claims_it() {
    let found = identify("src/main.rs", None).expect("rust");
    assert_eq!(found.language.id, "rust");
    assert_eq!(found.evidence, Evidence::Extension("rs".into()));

    let found = identify("Dockerfile", None).expect("dockerfile");
    assert_eq!(found.language.id, "dockerfile");
}

#[test]
fn a_contested_name_with_no_primary_claim_says_so_rather_than_guessing() {
    // `.1` is claimed by more than one language and none of them claims it
    // primarily, so there is no honest answer from the name alone.
    let undecided = identify("man/git.1", None).expect_err("contested");
    match undecided {
        Undecided::Contested {
            extension,
            claimants,
            had_rules,
        } => {
            assert_eq!(extension, "1");
            assert!(claimants.len() > 1, "{claimants:?}");
            assert!(had_rules, "and linguist publishes rules for it");
        }
        other => panic!("expected a contest, got {other:?}"),
    }
}

#[test]
fn a_primary_claim_settles_a_contested_name_when_there_are_no_bytes_to_read() {
    // Three languages claim `.h`, but C claims it primarily, which is the
    // answer a walker that has not opened the file should get. This is a
    // deliberate choice and not an accident of ordering, so it is pinned.
    let found = identify("legacy.h", None).expect("c by primary claim");
    assert_eq!(found.language.id, "c");
    assert_eq!(found.evidence, Evidence::Extension("h".into()));
    assert_eq!(langbank::languages_claiming_extension("h").len(), 3);
}

#[test]
fn reading_the_file_settles_what_the_name_cannot() {
    let objc = "#import <Foundation/Foundation.h>\n@interface Greeter : NSObject\n@end\n";
    let cpp = "#pragma once\n#include <vector>\ntemplate <typename T>\nclass Holder {};\n";
    let c = "#ifndef GREET_H\n#define GREET_H\nvoid greet(void);\n#endif\n";

    for (content, expected) in [(objc, "objective-c"), (cpp, "cpp"), (c, "c")] {
        let found = identify("legacy.h", Some(content)).expect("settled by content");
        assert_eq!(found.language.id, expected, "content: {content:?}");
        assert!(matches!(found.evidence, Evidence::Content { .. }));
    }
}

#[test]
fn the_rule_that_fired_is_reported_not_just_the_answer() {
    let objc = "@interface Greeter : NSObject\n@end\n";
    let found = identify("legacy.h", Some(objc)).expect("objective-c");
    match found.evidence {
        // Objective-C is the first rule for `.h`; C is the fallback at the end.
        Evidence::Content { extension, rule } => {
            assert_eq!(extension, "h");
            assert_eq!(rule, 0);
        }
        other => panic!("expected content evidence, got {other:?}"),
    }
    let fallback = identify("legacy.h", Some("void f(void);\n")).expect("c");
    match fallback.evidence {
        Evidence::Content { rule, .. } => assert!(rule > 0, "the fallback is not the first rule"),
        other => panic!("expected content evidence, got {other:?}"),
    }
}

#[test]
fn a_shebang_names_a_language_with_no_extension_at_all() {
    let found = identify("scripts/deploy", Some("#!/usr/bin/env bash\necho hi\n")).expect("shell");
    assert_eq!(found.language.id, "shell");
    assert!(matches!(found.evidence, Evidence::Shebang(_)));
}

#[test]
fn nothing_known_is_reported_as_nothing_known() {
    assert_eq!(
        identify("notes.qqzz", None).unwrap_err(),
        Undecided::Unknown
    );
    assert_eq!(identify("README", None).unwrap_err(), Undecided::Unknown);
}

#[test]
fn every_rule_in_the_registry_compiles_or_is_marked_unportable() {
    // The claim langbank makes about its own data, checked by compiling it.
    // Six rules of 317 are flagged unportable; every other pattern must build.
    // Three of those six were found by this test: `portable` used to be decided
    // by grepping for lookaround, which missed an unescaped `{` that Ruby reads
    // as a literal and Rust reads as a malformed quantifier.
    let mut unportable_that_compile = 0;
    for block in langbank::disambiguations() {
        for rule in block.rules {
            for clause in rule.clauses {
                for pattern in clause.patterns.iter().chain(clause.negative.iter()) {
                    let ok = regex_compiles(pattern);
                    if rule.portable {
                        assert!(
                            ok,
                            "{} claims portable but {pattern:?} does not compile",
                            rule.language.id
                        );
                    } else if ok {
                        unportable_that_compile += 1;
                    }
                }
            }
        }
    }
    // Not an assertion that they all fail — `portable` is per rule and a rule
    // may mix patterns — only that the flag is not hiding a broken majority.
    assert!(unportable_that_compile < 20, "{unportable_that_compile}");
}

fn regex_compiles(pattern: &str) -> bool {
    regex::Regex::new(pattern).is_ok()
}

#[test]
fn a_rule_matches_on_a_line_that_is_not_the_first() {
    // Ruby anchors `^` to every line; Rust anchors it to the haystack unless
    // asked otherwise. Getting this wrong silently loses most of the 317 rules
    // rather than failing loudly, so the difference is pinned here: the C++
    // marker is deliberately on line 3.
    let cpp = "// a header\n#pragma once\ntemplate <typename T>\nclass Holder {};\n";
    let found = identify("legacy.h", Some(cpp)).expect("settled by content");
    assert_eq!(found.language.id, "cpp");
}
