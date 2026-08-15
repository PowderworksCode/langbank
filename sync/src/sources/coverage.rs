//! How much langbank knows about each language.
//!
//! Reads nothing but langbank itself. The point is that the gaps are a
//! distribution rather than a feeling: most languages know exactly one thing
//! about themselves, and that is worth seeing rather than inferring.

use std::collections::BTreeMap;

use langbank::*;

use crate::report::{Outcome, Result};

const FACETS: [&str; 8] = [
    "detection",
    "comments",
    "facets",
    "conventions",
    "toolchain",
    "compiler",
    "analyser",
    "ecosystem",
];

/// Rounded to nearest, not truncated: the Python this replaced used `round`,
/// and a bar one character short is exactly the sort of difference that makes
/// somebody wonder whether the port changed anything else.
fn scaled(width: usize, part: usize, whole: usize) -> usize {
    let whole = whole.max(1);
    (width * part + whole / 2) / whole
}

fn known(profile: &'static LanguageProfile) -> [bool; 8] {
    let serves = toolchains_for(profile);
    let is = |kind| serves.iter().any(|entry| entry.is(kind));
    [
        !profile.extensions.is_empty()
            || !profile.filenames.is_empty()
            || !profile.shebangs.is_empty(),
        profile.comments.is_some(),
        !profile.facets.is_empty(),
        profile.conventions.is_some(),
        !serves.is_empty(),
        is(ToolchainKind::Compiler) || is(ToolchainKind::Runtime),
        is(ToolchainKind::Linter) || is(ToolchainKind::Formatter),
        ecosystem_profiles()
            .iter()
            .any(|ecosystem| ecosystem.implies_language(profile)),
    ]
}

pub fn run(arguments: &[String]) -> Result<Outcome> {
    let detail = arguments.iter().any(|argument| argument == "--detail");
    let profiles = language_profiles();
    let rows = profiles
        .iter()
        .map(|profile| (profile.id, known(profile)))
        .collect::<Vec<_>>();

    if detail {
        for (id, marks) in &rows {
            let line = marks
                .iter()
                .map(|have| if *have { 'x' } else { '.' })
                .collect::<String>();
            println!("  {line}  {id}");
        }
        return Ok(Outcome::Complete);
    }

    println!("{} languages\n", rows.len());
    println!("{:<14} {:>6} {:>6}   share", "facet", "have", "lack");
    for (index, facet) in FACETS.iter().enumerate() {
        let have = rows.iter().filter(|(_, marks)| marks[index]).count();
        let bar = "#".repeat(scaled(40, have, rows.len()));
        println!("{facet:<14} {have:>6} {:>6}   {bar}", rows.len() - have);
    }

    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for (_, marks) in &rows {
        *histogram
            .entry(marks.iter().filter(|have| **have).count())
            .or_default() += 1;
    }
    println!("\nfacets known, by language count:");
    for (score, count) in &histogram {
        let bar = "#".repeat(scaled(60, *count, rows.len()));
        println!("  {score} of {}: {count:>4}  {bar}", FACETS.len());
    }
    Ok(Outcome::Complete)
}
