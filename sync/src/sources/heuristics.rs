//! Linguist's content heuristics: ordered rules that name a language by reading
//! a file, for extensions several languages claim.
//!
//! Ported from `tools/sync-heuristics.py`, with one deliberate change. The
//! Python decided `portable` by looking for lookaround and backreferences —
//! a guess about what Rust's regex rejects, made in a language that cannot ask
//! it. This asks it: every pattern is handed to `regex::Regex::new`, and
//! `portable` is whether it came back. That found three rules the guess missed,
//! two of them an unescaped `{` that Ruby reads as a literal and Rust reads as
//! a malformed quantifier.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use serde::Deserialize;
use std::collections::BTreeMap;

const UPSTREAM: &str =
    "https://raw.githubusercontent.com/github-linguist/linguist/main/lib/linguist/heuristics.yml";
const OUT: &str = "data/heuristics.toml";

#[derive(Deserialize)]
struct Document {
    disambiguations: Vec<Block>,
    #[serde(default, rename = "named_patterns")]
    named: BTreeMap<String, Listy>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    rules: Vec<Rule>,
}

/// Linguist writes a clause six ways; `and` nests one level and never more.
#[derive(Deserialize)]
struct Rule {
    #[serde(default)]
    language: Option<Listy>,
    #[serde(default)]
    pattern: Option<Listy>,
    #[serde(default, rename = "named_pattern")]
    named_pattern: Option<String>,
    #[serde(default, rename = "negative_pattern")]
    negative: Option<Listy>,
    #[serde(default)]
    and: Vec<Rule>,
}

/// A YAML field that is a string, a list of strings, or absent.
#[derive(Deserialize)]
#[serde(untagged)]
enum Listy {
    One(String),
    Many(Vec<String>),
}

impl Listy {
    fn all(value: &Option<Listy>) -> Vec<String> {
        match value {
            None => vec![],
            Some(Listy::One(s)) => vec![s.clone()],
            Some(Listy::Many(v)) => v.clone(),
        }
    }
}

struct Clause {
    patterns: Vec<String>,
    negative: Vec<String>,
}

struct Carried {
    language: String,
    clauses: Vec<Clause>,
    portable: bool,
}

struct Mapped {
    extensions: Vec<String>,
    rules: Vec<Carried>,
}

/// Display name to langbank id, read from the data rather than a table, so a
/// language renamed in `data/languages/` is followed here without an edit.
fn languages() -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        let display = local::scalar(&text, "display-name").unwrap_or_else(|| id.clone());
        out.insert(display, id);
    }
    Ok(out)
}

fn clauses(rule: &Rule, named: &BTreeMap<String, Listy>) -> Vec<Clause> {
    let mut out = vec![];
    let mut one = |node: &Rule| {
        let mut patterns = Listy::all(&node.pattern);
        if let Some(key) = &node.named_pattern
            && let Some(found) = named.get(key)
        {
            patterns.extend(Listy::all(&Some(match found {
                Listy::One(s) => Listy::One(s.clone()),
                Listy::Many(v) => Listy::Many(v.clone()),
            })));
        }
        let negative = Listy::all(&node.negative);
        if !patterns.is_empty() || !negative.is_empty() {
            out.push(Clause { patterns, negative });
        }
    };
    one(rule);
    for sub in &rule.and {
        one(sub);
    }
    out
}

fn build(document: &Document, known: &BTreeMap<String, String>) -> (Vec<Mapped>, usize) {
    let (mut blocks, mut dropped) = (vec![], 0);
    for block in &document.disambiguations {
        let mut rules = Some(vec![]);
        for rule in &block.rules {
            let names = Listy::all(&rule.language);
            let mapped: Vec<&String> = names.iter().filter_map(|n| known.get(n)).collect();
            if mapped.len() != names.len() || mapped.is_empty() {
                // Dropping one rule would shift the ones after it, and these
                // are evaluated in order, so the whole block goes.
                dropped += 1;
                rules = None;
                break;
            }
            let conditions = clauses(rule, &document.named);
            let portable = conditions
                .iter()
                .flat_map(|c| c.patterns.iter().chain(c.negative.iter()))
                .all(|p| regex::Regex::new(p).is_ok());
            if let Some(rules) = rules.as_mut() {
                rules.push(Carried {
                    language: mapped[0].clone(),
                    clauses: conditions,
                    portable,
                });
            }
        }
        if let Some(rules) = rules
            && !rules.is_empty()
        {
            blocks.push(Mapped {
                extensions: block
                    .extensions
                    .iter()
                    .map(|e| e.trim_start_matches('.').to_lowercase())
                    .collect(),
                rules,
            });
        }
    }
    (blocks, dropped)
}

fn render(blocks: &[Mapped]) -> String {
    let mut lines = vec![
        "# @generated by `cargo run -p langbank-sync -- heuristics create` from".to_string(),
        "# github-linguist/linguist (MIT).".to_string(),
        "#".to_string(),
        "# Ordered rules that name a language by reading a file, for extensions".to_string(),
        "# several languages claim. Langbank does not read files; it carries the".to_string(),
        "# rules so a consumer that has the bytes can settle what langbank cannot.".to_string(),
        "# First rule whose clauses all match wins. A rule with no clauses always".to_string(),
        "# matches, which is how a fallback is spelled.".to_string(),
        "#".to_string(),
        "# `portable = false` means the pattern does not compile under Rust's regex".to_string(),
        "# crate — checked by compiling it, not by looking for constructs.".to_string(),
        String::new(),
    ];
    for block in blocks {
        lines.push("[[disambiguation]]".into());
        lines.push(format!(
            "extensions = {}",
            local::toml_array(&block.extensions)
        ));
        for rule in &block.rules {
            lines.push(String::new());
            lines.push("  [[disambiguation.rule]]".into());
            lines.push(format!(
                "  language = {}",
                local::toml_string(&rule.language)
            ));
            if !rule.portable {
                lines.push("  portable = false".into());
            }
            for clause in &rule.clauses {
                lines.push(String::new());
                lines.push("    [[disambiguation.rule.clause]]".into());
                lines.push(format!(
                    "    patterns = {}",
                    local::toml_array(&clause.patterns)
                ));
                if !clause.negative.is_empty() {
                    lines.push(format!(
                        "    negative = {}",
                        local::toml_array(&clause.negative)
                    ));
                }
            }
        }
        lines.push(String::new());
    }
    lines.join("\n") + "\n"
}

pub fn run(verb: &str) -> Result<Outcome> {
    let document: Document = serde_yaml_ng::from_str(&fetch::text(UPSTREAM)?)
        .map_err(|e| format!("linguist heuristics.yml did not parse: {e}"))?;
    let published = document.disambiguations.len();
    let (blocks, dropped) = build(&document, &languages()?);
    let rules: usize = blocks.iter().map(|b| b.rules.len()).sum();
    let unportable: usize = blocks
        .iter()
        .flat_map(|b| b.rules.iter())
        .filter(|r| !r.portable)
        .count();

    if verb == "check" {
        let carried = std::fs::read_to_string(OUT).unwrap_or_default();
        let current = carried.matches("[[disambiguation]]").count();
        println!(
            "linguist publishes {published} disambiguation blocks; {} map onto langbank \
             languages, {dropped} do not",
            blocks.len()
        );
        println!("  langbank carries {current}");
        println!("  rules: {rules}, of which {unportable} do not compile under Rust's regex");
        return Ok(Outcome::of(blocks.len().abs_diff(current)));
    }

    std::fs::write(OUT, render(&blocks)).map_err(|e| format!("{OUT}: {e}"))?;
    println!(
        "wrote {} disambiguation blocks, {rules} rules ({unportable} not portable to Rust \
         regex); {dropped} blocks dropped for naming a language langbank does not carry",
        blocks.len()
    );
    Ok(Outcome::of(0))
}
