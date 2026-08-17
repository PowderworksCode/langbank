//! The inline-test rules, actually run.
//!
//! These are hand-written conventions rather than absorbed facts, which makes
//! them exactly the kind of data that is easy to get subtly wrong and never
//! find out. `inline_test_indicator` is the thing that reads them, so it is the
//! thing this exercises.

use langbank::language_profile;

/// A line that should fire, and a line from the same language that should not.
const SAMPLES: &[(&str, &str, &str)] = &[
    (
        "go",
        "func TestParse(t *testing.T) {",
        "func Parse(input string) error {",
    ),
    ("go", "func BenchmarkParse(b *testing.B) {", "package main"),
    ("python", "import pytest", "import os"),
    (
        "python",
        "from unittest import TestCase",
        "from os import path",
    ),
    (
        "python",
        "def test_parses_a_header():",
        "def parse(header):",
    ),
    ("java", "@Test", "@Override"),
    ("kotlin", "@Test", "@Composable"),
    ("ruby", "RSpec.describe Parser do", "class Parser"),
    ("elixir", "use ExUnit.Case", "use GenServer"),
    (
        "php",
        "use PHPUnit\\Framework\\TestCase;",
        "use App\\Parser;",
    ),
    ("swift", "import XCTest", "import Foundation"),
    (
        "dart",
        "import 'package:test/test.dart';",
        "import 'dart:io';",
    ),
    ("c-sharp", "[Fact]", "[Serializable]"),
    ("c-sharp", "[Test]", "public class Parser"),
    (
        "zig",
        "test \"parses a header\" {",
        "const std = @import(\"std\");",
    ),
    ("rust", "#[test]", "#[derive(Debug)]"),
    (
        "typescript",
        "describe(\"parser\", () => {",
        "export function parse() {}",
    ),
];

#[test]
fn every_inline_test_rule_fires_on_its_own_example() {
    for (id, positive, negative) in SAMPLES {
        let profile = language_profile(id).unwrap_or_else(|| panic!("{id} is carried"));
        let conventions = profile
            .conventions
            .unwrap_or_else(|| panic!("{id} has conventions"));
        assert!(
            conventions.inline_test_indicator(positive).is_some(),
            "{id}: {positive:?} should be recognised as an inline test"
        );
        assert!(
            conventions.inline_test_indicator(negative).is_none(),
            "{id}: {negative:?} should NOT be recognised as an inline test, but {:?} matched",
            conventions.inline_test_indicator(negative)
        );
    }
}

#[test]
fn a_test_layout_names_somewhere_a_test_could_be() {
    for profile in langbank::language_profiles() {
        let Some(conventions) = profile.conventions else {
            continue;
        };
        let layout = &conventions.test_layout;
        // Either tests live in their own root, or they sit beside the source
        // under a suffix. A layout with neither describes nothing.
        assert!(
            !layout.test_root.is_empty() || !layout.test_suffixes.is_empty(),
            "{} has a test layout that locates nothing",
            profile.id
        );
        for suffix in layout.test_suffixes {
            assert!(
                !suffix.contains('/') && !suffix.contains('*'),
                "{}: {suffix:?} is a path or a glob, not a suffix",
                profile.id
            );
        }
    }
}

#[test]
fn every_language_with_an_inline_rule_can_report_it() {
    // A rule whose `indicator` is empty matches and then says nothing, which is
    // worse than not matching.
    for profile in langbank::language_profiles() {
        let Some(conventions) = profile.conventions else {
            continue;
        };
        for rule in conventions.inline_test {
            assert!(
                !rule.indicator.is_empty(),
                "{} has a silent rule",
                profile.id
            );
            assert!(
                !rule.starts_with.is_empty(),
                "{} has a rule that starts with nothing and matches every line",
                profile.id
            );
        }
    }
}
