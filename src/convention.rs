use crate::LanguageProfile;

#[derive(Debug, Clone, Copy)]
pub struct TestLayoutDefaults {
    pub source_roots: &'static [&'static str],
    pub test_root: &'static str,
    pub test_suffixes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct TypecheckConvention {
    pub config_files: &'static [&'static str],
}

/// One way a file announces that it contains its own tests.
///
/// This was a function pointer per language, which is why the profiles could
/// not be data. Both detectors that existed were the same shape — a line
/// prefix, sometimes narrowed by something the same line must also contain —
/// so the shape is the rule and the languages differ only in their tables.
#[derive(Debug, Clone, Copy)]
pub struct InlineTestRule {
    /// The line, trimmed of leading whitespace, must start with one of these.
    pub starts_with: &'static [&'static str],
    /// When non-empty, the same line must also contain one of these. This is
    /// what separates `import { test } from "vitest"` from any other import.
    pub contains_any: &'static [&'static str],
    /// What to report when the rule matches.
    pub indicator: &'static str,
}

impl InlineTestRule {
    fn matches(&self, line: &str) -> Option<&'static str> {
        if !self
            .starts_with
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            return None;
        }
        if !self.contains_any.is_empty()
            && !self.contains_any.iter().any(|needle| line.contains(needle))
        {
            return None;
        }
        Some(self.indicator)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageConventions {
    pub typecheck: Option<TypecheckConvention>,
    pub test_layout: TestLayoutDefaults,
    pub inline_test: &'static [InlineTestRule],
}

impl LanguageConventions {
    /// What in this content says it holds its own tests, if anything.
    ///
    /// Line-major then rule-major, first match wins — the same order the
    /// hand-written detectors evaluated in, so the answer does not change.
    pub fn inline_test_indicator(&self, content: &str) -> Option<&'static str> {
        content.lines().find_map(|line| {
            let line = line.trim_start();
            self.inline_test.iter().find_map(|rule| rule.matches(line))
        })
    }
}

pub fn language_conventions(language: &LanguageProfile) -> Option<&LanguageConventions> {
    language.conventions.as_ref()
}
