//! Run every version probe against whatever is installed here.
//!
//! Langbank states how to read a program's version and never runs it, so
//! nothing in the crate can tell whether a pattern still matches what the
//! program prints. This can. Absent programs are skipped rather than failed —
//! no machine has all of them — which is why the strict flag exists for CI,
//! where a probe that fails *on a runner that has the tool* is a real break.

use std::process::Command;

use langbank::*;
use regex::Regex;

use crate::report::{Outcome, Result};

fn installed(program: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(program).is_file())
}

pub fn run(arguments: &[String]) -> Result<Outcome> {
    let strict = arguments.iter().any(|argument| argument == "--strict");
    let (mut verified, mut skipped, mut failed) = (0usize, 0usize, 0usize);

    for entry in toolchains() {
        let Some(probe) = entry.version else { continue };
        let Some(program) = entry.programs.iter().find(|program| installed(program)) else {
            println!(
                "  {:<10} skipped   (none of {} installed)",
                entry.id,
                entry.programs.join(" ")
            );
            skipped += 1;
            continue;
        };
        let output = match Command::new(program).args(probe.arguments).output() {
            Ok(output) => output,
            Err(error) => {
                println!("  {:<10} FAILED    running {program}: {error}", entry.id);
                failed += 1;
                continue;
            }
        };
        let stream = match probe.stream {
            OutputStream::Stdout => output.stdout,
            OutputStream::Stderr => output.stderr,
        };
        let text = String::from_utf8_lossy(&stream);
        let line = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or_default();
        let pattern = Regex::new(probe.pattern)?;
        match pattern.captures(line).and_then(|found| found.get(1)) {
            Some(version) => {
                println!(
                    "  {:<10} verified  {:<12} via {program}",
                    entry.id,
                    version.as_str()
                );
                verified += 1;
            }
            None => {
                println!(
                    "  {:<10} FAILED    pattern did not match {line:?}",
                    entry.id
                );
                failed += 1;
            }
        }
    }

    println!("\n{verified} verified, {skipped} skipped, {failed} failed");
    Ok(if failed > 0 && strict {
        Outcome::Incomplete
    } else {
        Outcome::Complete
    })
}
