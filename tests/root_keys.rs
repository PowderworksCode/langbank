//! Root-level keys must be at the root.
//!
//! A `key = value` written after a `[table]` header belongs to that table, and
//! the document still parses, so nothing complains. `build.rs` reads these at
//! the root, finds nothing, and emits `None` — no error anywhere, just a field
//! that is quietly always empty.
//!
//! `gcc.toml` carried its `categories` under `[diagnostics]` that way for
//! months. Adding homepages by the same append put 485 of them somewhere no
//! reader would look, which is how it was finally noticed.

use std::collections::BTreeSet;

/// Keys that are meaningful only at the root of an entry.
const ROOT_ONLY: [&str; 6] = [
    "id",
    "display-name",
    "categories",
    "homepage",
    "repository",
    "kind",
];

#[test]
fn no_entry_hides_a_root_key_inside_a_table() {
    let mut offences = Vec::new();
    // `data/sources/` is a pin file, where `[linguist] repository = ...` is the
    // intended shape rather than a misplaced key.
    for directory in [
        "languages",
        "toolchains",
        "ecosystems",
        "registries",
        "tools",
    ] {
        let pattern = format!("data/{directory}");
        let Ok(entries) = std::fs::read_dir(&pattern) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read entry");
            let document: toml::Value =
                toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let Some(table) = document.as_table() else {
                continue;
            };
            for (name, value) in table {
                let Some(nested) = value.as_table() else {
                    continue;
                };
                let hidden: BTreeSet<&str> = ROOT_ONLY
                    .into_iter()
                    .filter(|key| nested.contains_key(*key))
                    .collect();
                if !hidden.is_empty() {
                    offences.push(format!(
                        "{}: {hidden:?} sits under [{name}] and will read as absent",
                        path.display()
                    ));
                }
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn the_leaf_touches_no_filesystem() {
    // The README says so in its second paragraph, and one function did anyway:
    // `EcosystemProfile::lockfile_present` stat'd a directory. Nothing called
    // it, so nothing failed — the claim was just quietly untrue. Walking a
    // directory is `langbank-detect`'s job.
    let mut offences = Vec::new();
    for entry in std::fs::read_dir("src").expect("read src") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read source");
        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or(line);
            for probe in [
                "is_file()",
                "is_dir()",
                "read_dir(",
                "File::open",
                "fs::read",
                "fs::write",
            ] {
                if code.contains(probe) {
                    offences.push(format!("{}:{}: {probe}", path.display(), number + 1));
                }
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}
