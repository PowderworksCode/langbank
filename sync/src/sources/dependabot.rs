//! Package ecosystems from dependabot-core.
//!
//! Ported from `tools/sync-dependabot.py`. It reads the JSON that
//! `tools/extract-dependabot.rb` emits — see the note on that script below.
//!
//! What dependabot does not say is which language an ecosystem publishes for,
//! or which purl registry its packages live in. Those are the table here,
//! written by hand because they are judgements rather than extractions, and
//! each one is a claim somebody should be able to check.
//!
//! Ecosystems dependabot updates that are not language package managers —
//! github_actions, git_submodules, devcontainers, pre_commit, terraform,
//! docker — are deliberately absent. Langbank's ecosystem is a thing that
//! manages a language's packages.
//!
//! ## Why the extractor is still Ruby
//!
//! Dependabot states its facts as code — `filenames.include?("Cargo.toml")` —
//! and several ecosystems name a constant instead of spelling a filename out:
//! composer says `PackageManager::MANIFEST_FILENAME`, deno says
//! `MANIFEST_FILENAMES`. Following those references took the yield from 16
//! ecosystems to 27, and doing it correctly needs a parse, not a pattern.
//! Ripper is stdlib, is the parser Ruby itself uses, and is right by
//! construction. Replacing it with a hand-written reader in Rust would
//! reintroduce precisely the guessing at string boundaries, interpolation and
//! comments that it exists to avoid, so the extractor stays where it is and
//! this reads its output.

use crate::local;
use crate::report::{Outcome, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// slug -> (id, display name, languages, purl registry, roles)
struct Mapping {
    slug: &'static str,
    id: &'static str,
    display: &'static str,
    languages: &'static [&'static str],
    registry: Option<&'static str>,
    roles: &'static [&'static str],
}

const KNOWN: &[Mapping] = &[
    Mapping {
        slug: "bazel",
        id: "bazel",
        display: "Bazel",
        languages: &[],
        registry: Some("bazel"),
        roles: &["build-system"],
    },
    Mapping {
        slug: "bundler",
        id: "bundler",
        display: "Bundler",
        languages: &["ruby"],
        registry: Some("gem"),
        roles: &["package-manager"],
    },
    Mapping {
        slug: "composer",
        id: "composer",
        display: "Composer",
        languages: &["php"],
        registry: Some("composer"),
        roles: &["package-manager"],
    },
    Mapping {
        slug: "conda",
        id: "conda",
        display: "conda",
        languages: &["python"],
        registry: Some("conda"),
        roles: &["package-manager"],
    },
    Mapping {
        slug: "deno",
        id: "deno",
        display: "Deno",
        languages: &["typescript", "javascript"],
        registry: None,
        roles: &["package-manager", "runtime"],
    },
    Mapping {
        slug: "elm",
        id: "elm",
        display: "Elm packages",
        languages: &["elm"],
        registry: None,
        roles: &["package-manager"],
    },
    Mapping {
        slug: "go_modules",
        id: "go-modules",
        display: "Go modules",
        languages: &["go"],
        registry: Some("golang"),
        roles: &["package-manager", "build-system"],
    },
    Mapping {
        slug: "gradle",
        id: "gradle",
        display: "Gradle",
        languages: &["java", "kotlin"],
        registry: Some("maven"),
        roles: &["package-manager", "build-system"],
    },
    Mapping {
        slug: "hex",
        id: "hex",
        display: "Hex",
        languages: &["elixir"],
        registry: Some("hex"),
        roles: &["package-manager"],
    },
    Mapping {
        slug: "maven",
        id: "maven",
        display: "Maven",
        languages: &["java"],
        registry: Some("maven"),
        roles: &["package-manager", "build-system"],
    },
    Mapping {
        slug: "pub",
        id: "pub",
        display: "Pub",
        languages: &["dart"],
        registry: Some("pub"),
        roles: &["package-manager"],
    },
    Mapping {
        slug: "sbt",
        id: "sbt",
        display: "sbt",
        languages: &["scala"],
        registry: Some("maven"),
        roles: &["package-manager", "build-system"],
    },
    Mapping {
        slug: "swift",
        id: "swift-pm",
        display: "Swift Package Manager",
        languages: &["swift"],
        registry: Some("swift"),
        roles: &["package-manager", "build-system"],
    },
];

#[derive(Deserialize, Default)]
struct Facts {
    slug: String,
    #[serde(default)]
    required_files: Vec<String>,
    #[serde(default)]
    lockfiles: Vec<String>,
}

/// A literal filename, not a regex dependabot matches with.
fn plain(name: &str) -> bool {
    !name.contains('/')
        && !name.chars().any(|c| {
            matches!(
                c,
                '\\' | '^' | '$' | '*' | '+' | '?' | '[' | ']' | '(' | ')' | '|'
            )
        })
}

fn ids(directory: &str) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for path in local::files(directory)? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(id) = local::scalar(&text, "id") {
            out.insert(id);
        }
    }
    Ok(out)
}

fn render(entry: &Mapping, facts: &Facts) -> String {
    let manifests: Vec<&String> = facts.required_files.iter().filter(|f| plain(f)).collect();
    let locks: Vec<&String> = facts.lockfiles.iter().filter(|f| plain(f)).collect();

    let mut lines = vec![format!("id = \"{}\"", entry.id)];
    if let Some(registry) = entry.registry {
        lines.push(format!("registry = \"{registry}\""));
    }
    lines.push(format!(
        "display-name = {}",
        local::toml_string(entry.display)
    ));
    lines.push(format!("roles = {}", local::toml_array(entry.roles)));
    if !entry.languages.is_empty() {
        lines.push(format!(
            "implied-languages = {}",
            local::toml_array(entry.languages)
        ));
    }
    if let Some(first) = manifests.first() {
        // Langbank models one manifest per ecosystem; the rest of what
        // dependabot accepts is recorded as a selector so nothing is lost.
        lines.push(format!("manifest = {}", local::toml_string(first)));
        if manifests.len() > 1 {
            lines.push(format!(
                "selector-files = {}",
                local::toml_array(&manifests[1..])
            ));
        }
    }
    if !locks.is_empty() {
        lines.push(format!("lockfiles = {}", local::toml_array(&locks)));
    }
    lines.push("manifest-selection = \"default\"".to_string());
    lines.join("\n") + "\n"
}

pub fn run(verb: &str, path: Option<&str>) -> Result<Outcome> {
    let path = path.ok_or("usage: langbank-sync dependabot <check|create> <facts.json>")?;
    let raw = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let parsed: Vec<Facts> =
        serde_json::from_str(&raw).map_err(|e| format!("{path} did not parse: {e}"))?;
    let facts: BTreeMap<String, Facts> = parsed
        .into_iter()
        .map(|entry| (entry.slug.clone(), entry))
        .collect();

    let have = ids("data/ecosystems")?;
    let known_languages = ids("data/languages")?;
    let known_registries = ids("data/registries")?;

    let (mut missing, mut bad) = (Vec::new(), Vec::new());
    for entry in KNOWN {
        if !facts.contains_key(entry.slug) {
            bad.push(format!(
                "{}: dependabot no longer defines this ecosystem",
                entry.slug
            ));
            continue;
        }
        for language in entry.languages {
            if !known_languages.contains(*language) {
                bad.push(format!("{}: unknown language '{language}'", entry.id));
            }
        }
        if let Some(registry) = entry.registry
            && !known_registries.contains(registry)
        {
            bad.push(format!("{}: '{registry}' is not a purl type", entry.id));
        }
        if !have.contains(entry.id) {
            missing.push(entry);
        }
    }

    if verb == "check" {
        println!(
            "{} package ecosystems mapped from dependabot; {} carried, {} missing",
            KNOWN.len(),
            have.len(),
            missing.len()
        );
        for problem in &bad {
            println!("  {problem}");
        }
        return Ok(Outcome::of(missing.len() + bad.len()));
    }

    // A bad mapping is not written past. An ecosystem naming a language or a
    // registry langbank does not carry would compile into a dangling reference,
    // and the build would fail somewhere with no mention of dependabot.
    if !bad.is_empty() {
        for problem in &bad {
            println!("  {problem}");
        }
        return Ok(Outcome::Incomplete);
    }
    for entry in &missing {
        let Some(entry_facts) = facts.get(entry.slug) else {
            continue;
        };
        let path = format!("data/ecosystems/{}.toml", entry.id);
        std::fs::write(&path, render(entry, entry_facts)).map_err(|e| format!("{path}: {e}"))?;
    }
    println!("created {} ecosystems", missing.len());
    Ok(Outcome::Complete)
}
