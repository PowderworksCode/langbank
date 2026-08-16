//! Keeps langbank's data in step with the upstreams it is checked against.
//!
//! Every subcommand takes the same two verbs. `check` reports what an upstream
//! knows and langbank does not, and is what `drift.yml` runs; `create` writes
//! the files that are missing and never touches one that already exists,
//! because a hand-written entry carries judgements no importer should overrule.
//!
//! These were Python until they were Rust. They live beside the crate rather
//! than in a scripts directory because they are the same knowledge: the rules
//! for reading an upstream are as much a fact about it as the data they yield.

mod fetch;
mod local;
mod report;
mod sources;

use std::process::ExitCode;

use report::Outcome;

const USAGE: &str = "\
langbank-sync — keep langbank's data in step with its upstreams

usage: langbank-sync <source> <check|create> [options]

sources:
  linguist          languages, extensions, filenames, interpreters
  corpora           comment syntax and extensions, from tokei and scc
  purl              package registries and their identity rules
  heuristics        content rules for extensions a name cannot settle
  toolchains        run every version probe against what is installed here
  coverage          report how much langbank knows about each language

every source takes:
  check             report what is missing; non-zero when something is
  create            write what is missing, never overwriting what exists
";

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    // `coverage` and `toolchains` read what is already here and take no verb;
    // the sources that talk to an upstream need one.
    let (source, verb) = match arguments.as_slice() {
        [source, verb, ..] => (source.as_str(), verb.as_str()),
        [source] => (source.as_str(), "check"),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let outcome = match (source, verb) {
        ("linguist", verb) => sources::linguist::run(verb),
        ("corpora", verb) => sources::corpora::run(verb),
        ("purl", verb) => sources::purl::run(verb),
        ("heuristics", verb) => sources::heuristics::run(verb),
        ("toolchains", _) => sources::toolchains::run(&arguments),
        ("coverage", _) => sources::coverage::run(&arguments),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    match outcome {
        Ok(Outcome::Complete) => ExitCode::SUCCESS,
        Ok(Outcome::Incomplete) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("langbank-sync: {error}");
            ExitCode::from(2)
        }
    }
}
