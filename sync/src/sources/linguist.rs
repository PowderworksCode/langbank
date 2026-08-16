//! Languages, extensions, filenames and interpreters, from github-linguist.
//!
//! Ported from `tools/sync-linguist.py`. Langbank owns its language data;
//! linguist is one source it is checked against. The two verbs are kept apart
//! deliberately: `check` reports every language and token linguist knows and
//! langbank does not, and `create` writes a file only where none exists. A
//! hand-written profile carries conventions, facets and comment syntax no
//! importer has any business touching, so a token missing from an existing file
//! is reported for a person to add rather than merged in.

use crate::report::{Outcome, Result};
use crate::{fetch, local};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const UPSTREAM: &str = "github-linguist/linguist";
const PIN: &str = "data/sources/linguist.toml";

fn url(revision: &str) -> String {
    format!("https://raw.githubusercontent.com/{UPSTREAM}/{revision}/lib/linguist/languages.yml")
}

/// linguist's `type` is coarser than `LanguageRole` and never names build files.
fn role(kind: &str) -> Option<&'static str> {
    match kind {
        "programming" => Some("programming"),
        "markup" => Some("markup"),
        "data" => Some("data"),
        "prose" => Some("documentation"),
        _ => None,
    }
}

/// `C#` -> `c-sharp`, matching the ids the hand-written profiles use.
fn slug(name: &str) -> String {
    let lowered = name
        .to_lowercase()
        .replace('#', "-sharp")
        .replace("++", "pp")
        .replace('*', "-star");
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        out.push(if c.is_alphanumeric() { c } else { '-' });
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

#[derive(serde::Deserialize)]
struct Entry {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    filenames: Vec<String>,
    #[serde(default)]
    interpreters: Vec<String>,
}

struct Language {
    display: String,
    role: &'static str,
    extensions: Vec<String>,
    filenames: Vec<String>,
    shebangs: Vec<String>,
}

/// The pinned revision, or `main` when nothing is pinned yet.
fn pinned_revision() -> String {
    std::fs::read_to_string(PIN)
        .ok()
        .and_then(|text| local::scalar(&text, "revision"))
        .unwrap_or_else(|| "main".to_string())
}

fn upstream_languages(raw: &str) -> Result<BTreeMap<String, Language>> {
    let document: BTreeMap<String, Entry> = serde_yaml_ng::from_str(raw)
        .map_err(|error| format!("linguist languages.yml did not parse: {error}"))?;
    let mut out = BTreeMap::new();
    for (name, entry) in document {
        let Some(role) = entry.kind.as_deref().and_then(role) else {
            continue;
        };
        let extensions: BTreeSet<String> = entry
            .extensions
            .iter()
            .map(|e| e.trim_start_matches('.').to_lowercase())
            .collect();
        out.insert(
            slug(&name),
            Language {
                display: name,
                role,
                extensions: extensions.into_iter().collect(),
                filenames: entry
                    .filenames
                    .into_iter()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                shebangs: entry
                    .interpreters
                    .into_iter()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            },
        );
    }
    Ok(out)
}

struct Carried {
    extensions: BTreeSet<String>,
    filenames: BTreeSet<String>,
    shebangs: BTreeSet<String>,
}

fn local_languages() -> Result<BTreeMap<String, Carried>> {
    let mut out = BTreeMap::new();
    for path in local::files("data/languages")? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(id) = local::scalar(&text, "id") else {
            continue;
        };
        out.insert(
            id,
            Carried {
                extensions: local::array(&text, "extensions").into_iter().collect(),
                filenames: local::array(&text, "filenames").into_iter().collect(),
                shebangs: local::array(&text, "shebangs").into_iter().collect(),
            },
        );
    }
    Ok(out)
}

/// The token kinds missing for one language: `("extensions", ["rs", "rlib"])`.
type Holes = Vec<(&'static str, Vec<String>)>;

/// What is in `upstream` and not in `local`, per language and per token kind.
fn gaps(
    upstream: &BTreeMap<String, Language>,
    local: &BTreeMap<String, Carried>,
) -> (Vec<String>, BTreeMap<String, Holes>) {
    let missing_languages: Vec<String> = upstream
        .keys()
        .filter(|id| !local.contains_key(*id))
        .cloned()
        .collect();

    let mut missing_tokens = BTreeMap::new();
    for (id, entry) in upstream {
        let Some(carried) = local.get(id) else {
            continue;
        };
        let mut holes = Vec::new();
        for (kind, theirs, ours) in [
            ("extensions", &entry.extensions, &carried.extensions),
            ("filenames", &entry.filenames, &carried.filenames),
            ("shebangs", &entry.shebangs, &carried.shebangs),
        ] {
            let hole: Vec<String> = theirs
                .iter()
                .filter(|token| !ours.contains(*token))
                .cloned()
                .collect();
            if !hole.is_empty() {
                holes.push((kind, hole));
            }
        }
        if !holes.is_empty() {
            missing_tokens.insert(id.clone(), holes);
        }
    }
    (missing_languages, missing_tokens)
}

/// sha2 0.11 returns an `Array` that does not implement `LowerHex`, and the
/// pin file records the digest as hex, so it is written out here rather than
/// taking a dependency on an encoder to turn 32 bytes into 64 characters.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn write_language(id: &str, entry: &Language) -> Result<()> {
    let mut lines = vec![
        format!("id = {}", local::toml_string(id)),
        format!("display-name = {}", local::toml_string(&entry.display)),
        format!("role = {}", local::toml_string(entry.role)),
    ];
    for (key, values) in [
        ("extensions", &entry.extensions),
        ("filenames", &entry.filenames),
        ("shebangs", &entry.shebangs),
    ] {
        if !values.is_empty() {
            lines.push(format!("{key} = {}", local::toml_array(values)));
        }
    }
    let path = format!("data/languages/{id}.toml");
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{path}: {e}"))?;
    Ok(())
}

pub fn run(verb: &str) -> Result<Outcome> {
    let revision = pinned_revision();
    let url = url(&revision);
    let raw = fetch::bytes(&url)?;
    let digest = hex(&Sha256::digest(&raw));
    let raw = String::from_utf8(raw).map_err(|e| format!("languages.yml is not UTF-8: {e}"))?;

    let upstream = upstream_languages(&raw)?;
    let local = local_languages()?;
    let (missing_languages, missing_tokens) = gaps(&upstream, &local);
    let short: String = revision.chars().take(12).collect();

    if verb == "create" {
        for id in &missing_languages {
            if let Some(entry) = upstream.get(id) {
                write_language(id, entry)?;
            }
        }
        std::fs::create_dir_all("data/sources").map_err(|e| format!("data/sources: {e}"))?;
        std::fs::write(
            PIN,
            format!(
                "# Upstream sources langbank is checked against. `langbank-sync linguist check`\n\
                 # fails if any of them knows a language or a detection token we do not.\n\
                 \n[linguist]\nrepository = \"{UPSTREAM}\"\nrevision = \"{revision}\"\n\
                 source = \"{url}\"\nsha256 = \"{digest}\"\nlicense = \"MIT\"\n\
                 languages = {}\n",
                upstream.len()
            ),
        )
        .map_err(|e| format!("{PIN}: {e}"))?;
        println!(
            "created {} language files from linguist@{short}",
            missing_languages.len()
        );
        if !missing_tokens.is_empty() {
            println!(
                "{} existing files are missing tokens; run `check` to see them",
                missing_tokens.len()
            );
        }
        return Ok(Outcome::Complete);
    }

    println!(
        "linguist@{short}: {} languages, langbank has {}",
        upstream.len(),
        local.len()
    );
    if missing_languages.is_empty() && missing_tokens.is_empty() {
        println!("coverage complete: langbank knows every language and token linguist does");
        return Ok(Outcome::Complete);
    }
    if !missing_languages.is_empty() {
        println!("\n{} languages missing:", missing_languages.len());
        for id in missing_languages.iter().take(40) {
            println!("  {id}");
        }
        if missing_languages.len() > 40 {
            println!("  … and {} more", missing_languages.len() - 40);
        }
    }
    if !missing_tokens.is_empty() {
        println!(
            "\n{} languages missing detection tokens:",
            missing_tokens.len()
        );
        for (id, holes) in &missing_tokens {
            for (kind, values) in holes {
                println!("  {id}: {kind} {}", values.join(" "));
            }
        }
    }
    Ok(Outcome::Incomplete)
}
