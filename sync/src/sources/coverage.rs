//! How much langbank knows about each language.
//!
//! Reads nothing but langbank itself. The point is that the gaps are a
//! distribution rather than a feeling: most languages know exactly one thing
//! about themselves, and that is worth seeing rather than inferring.
//!
//! What counts as knowing something lives in the leaf, as `langbank::Facet`,
//! because langbank.dev reports the same figures and a registry whose coverage
//! report disagrees with its own website has a worse problem than a thin facet.

use langbank::{Facet, Knowledge, coverage, coverage_by_role, distribution, language_profiles};

use crate::report::{Outcome, Result};

/// Rounded to nearest, not truncated: the Python this replaced used `round`,
/// and a bar one character short is exactly the sort of difference that makes
/// somebody wonder whether the port changed anything else.
fn scaled(width: usize, part: usize, whole: usize) -> usize {
    let whole = whole.max(1);
    (width * part + whole / 2) / whole
}

pub fn run(arguments: &[String]) -> Result<Outcome> {
    let total = language_profiles().len();

    if arguments.iter().any(|argument| argument == "--detail") {
        for profile in language_profiles() {
            let knowledge = Knowledge::of(profile);
            let line: String = knowledge
                .facets()
                .map(|(_, have)| if have { 'x' } else { '.' })
                .collect();
            println!("  {line}  {}", profile.id);
        }
        return Ok(Outcome::Complete);
    }

    println!("{total} languages\n");
    println!("{:<14} {:>6} {:>6}   share", "facet", "have", "lack");
    for (facet, have) in Facet::ALL.into_iter().zip(coverage()) {
        let bar = "#".repeat(scaled(40, have, total));
        println!("{:<14} {have:>6} {:>6}   {bar}", facet.name(), total - have);
    }

    // The plain total sets the wrong target on its own: `ecosystem 25/827`
    // reads as 802 languages to go and fill in, and 270 of them are data,
    // markup or documentation formats that will never have a package manager.
    println!("\nby role, so the denominator means something:");
    print!("{:<16} {:>6}", "role", "count");
    for facet in Facet::ALL {
        print!(" {:>10}", &facet.name()[..facet.name().len().min(10)]);
    }
    println!();
    for (role, count, carried) in coverage_by_role() {
        print!("{:<16} {count:>6}", format!("{role:?}").to_lowercase());
        for have in carried {
            print!(" {have:>10}");
        }
        println!();
    }

    println!("\nfacets known, by language count:");
    for (score, count) in distribution().into_iter().enumerate() {
        if count == 0 {
            continue;
        }
        let bar = "#".repeat(scaled(60, count, total));
        println!("  {score} of {}: {count:>4}  {bar}", Facet::ALL.len());
    }
    Ok(Outcome::Complete)
}
