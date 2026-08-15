//! Rules for reading a file to settle what an extension cannot.
//!
//! This is the same bargain as the version probes: langbank states the rule and
//! the consumer executes it. What changes is that 92 of the 127 extensions
//! langbank declines to name now come with instructions for naming them.

use langbank::*;

#[test]
fn the_hardest_contests_now_carry_rules() {
    // `.h` is C, C++ or Objective-C and no amount of filename inspection
    // settles it. Linguist settles it by reading, and now so can a consumer.
    let h = disambiguation_for("h").expect("h has rules");
    let languages = h
        .rules
        .iter()
        .map(|rule| rule.language.id)
        .collect::<Vec<_>>();
    assert!(languages.contains(&"objective-c"));
    assert!(languages.contains(&"cpp"));
    assert!(languages.contains(&"c"));

    // `.inc` is claimed by a dozen languages, which is why langbank declines it
    let inc = disambiguation_for("inc").expect("inc has rules");
    assert!(inc.rules.len() >= 6, "{} rules for .inc", inc.rules.len());
}

#[test]
fn rules_are_ordered_and_the_last_is_usually_a_fallback() {
    // Linguist evaluates in order and stops at the first match; a rule with no
    // clauses always matches, which is how the default is spelled.
    let h = disambiguation_for("h").expect("h");
    let last = h.rules.last().expect("at least one rule");
    assert!(last.clauses.is_empty(), "the last .h rule is the fallback");
    assert_eq!(last.language.id, "c");
    // and the ones before it are conditional
    assert!(
        h.rules[0]
            .clauses
            .iter()
            .any(|clause| !clause.patterns.is_empty())
    );
}

#[test]
fn a_clause_can_require_a_pattern_and_forbid_another() {
    let with_negatives = disambiguations()
        .iter()
        .flat_map(|block| block.rules.iter())
        .flat_map(|rule| rule.clauses.iter())
        .filter(|clause| !clause.negative.is_empty())
        .count();
    assert!(
        with_negatives > 0,
        "negative patterns survived the flattening"
    );
}

#[test]
fn rules_that_rust_cannot_compile_say_so() {
    // Three of 317 use lookaround, which Rust's regex crate rejects. A consumer
    // learning that here is better off than one learning it from a panic.
    let unportable = disambiguations()
        .iter()
        .flat_map(|block| block.rules.iter())
        .filter(|rule| !rule.portable)
        .count();
    assert!(
        (1..=10).contains(&unportable),
        "{unportable} unportable rules"
    );
    let total = disambiguations()
        .iter()
        .map(|b| b.rules.len())
        .sum::<usize>();
    assert!(total > 300, "{total} rules");
}

#[test]
fn every_rule_names_a_language_langbank_carries() {
    for block in disambiguations() {
        assert!(!block.extensions.is_empty(), "a block with no extensions");
        for rule in block.rules {
            assert!(
                language_profile(rule.language.id).is_some(),
                "{} is not carried",
                rule.language.id
            );
        }
    }
}

#[test]
fn rules_exist_precisely_where_langbank_declines_to_answer() {
    // An extension langbank resolves outright needs no rules, and one it
    // declines is exactly where they earn their place.
    let declined_with_rules = disambiguations()
        .iter()
        .flat_map(|block| block.extensions.iter())
        .filter(|extension| language_profile_for_extension(extension).is_none())
        .count();
    assert!(
        declined_with_rules > 50,
        "{declined_with_rules} declined extensions now carry rules"
    );
}

#[test]
fn a_declined_extension_offers_either_a_rule_or_a_reason() {
    // Between heuristics and gaps, an unanswered extension should rarely be
    // silent about why. This measures how often that holds rather than
    // asserting it always does.
    let contested: Vec<&str> = language_profiles()
        .iter()
        .flat_map(|profile| profile.extensions.iter().copied())
        .filter(|extension| {
            languages_claiming_extension(extension).len() > 1
                && language_profile_for_extension(extension).is_none()
        })
        .collect();
    let explained = contested
        .iter()
        .filter(|extension| {
            disambiguation_for(extension).is_some() || gap(extension, "extension-owner").is_some()
        })
        .count();
    assert!(
        explained * 2 > contested.len(),
        "only {explained} of {} declined extensions explain themselves",
        contested.len()
    );
}
