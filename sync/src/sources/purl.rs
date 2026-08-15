//! Package registries, from package-url/purl-spec.
//!
//! A purl type is where a package identity lives — `pkg:npm/lodash@4` — and is
//! not the tool that reads a lockfile. Keeping those apart is why this data is
//! separate from `data/ecosystems/`.

use std::collections::BTreeSet;

use crate::report::{Outcome, Result};
use crate::{fetch, local};

const INDEX: &str =
    "https://raw.githubusercontent.com/package-url/purl-spec/main/purl-types-index.json";
const TYPE: &str =
    "https://raw.githubusercontent.com/package-url/purl-spec/main/types/{name}-definition.json";

fn definition(name: &str) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(&fetch::text(
        &TYPE.replace("{name}", name),
    )?)?)
}

fn carried() -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for path in local::files("data/registries")? {
        let text = std::fs::read_to_string(&path)?;
        if let Some(id) = local::scalar(&text, "id") {
            out.insert(id);
        }
    }
    Ok(out)
}

fn component(definition: &serde_json::Value, key: &str) -> String {
    let part = definition.get(format!("{key}_definition"));
    let requirement = part
        .and_then(|part| part.get("requirement"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("optional");
    let sensitive = part
        .and_then(|part| part.get("case_sensitive"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    format!("\n[{key}]\nrequirement = \"{requirement}\"\ncase-sensitive = {sensitive}\n")
}

fn write(name: &str, definition: &serde_json::Value) -> Result<()> {
    let repository = definition.get("repository");
    let mut out = format!(
        "id = {}\ndisplay-name = {}\n",
        local::toml_string(name),
        local::toml_string(
            definition
                .get("type_name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(name)
        ),
    );
    if let Some(url) = repository
        .and_then(|repository| repository.get("default_repository_url"))
        .and_then(serde_json::Value::as_str)
    {
        out.push_str(&format!(
            "default-repository = {}\n",
            local::toml_string(url)
        ));
    }
    let uses = repository
        .and_then(|repository| repository.get("use_repository"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    out.push_str(&format!("uses-repository = {uses}\n"));
    for key in ["namespace", "name", "version"] {
        out.push_str(&component(definition, key));
    }
    std::fs::write(format!("data/registries/{name}.toml"), out)?;
    Ok(())
}

pub fn run(verb: &str) -> Result<Outcome> {
    let names: Vec<String> = serde_json::from_str(&fetch::text(INDEX)?)?;
    let have = carried()?;
    let missing = names
        .iter()
        .filter(|name| !have.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let extra = have
        .iter()
        .filter(|id| !names.contains(id))
        .cloned()
        .collect::<Vec<_>>();

    if verb == "create" {
        for name in &missing {
            write(name, &definition(name)?)?;
        }
        println!("created {} registry files from purl-spec", missing.len());
        return Ok(Outcome::Complete);
    }

    println!(
        "purl-spec defines {} types, langbank carries {}",
        names.len(),
        have.len()
    );
    for id in &extra {
        println!("  langbank has {id}, which purl does not define");
    }
    for name in &missing {
        println!("  missing: {name}");
    }
    if missing.is_empty() && extra.is_empty() {
        println!("coverage complete: langbank carries every purl type");
    }
    Ok(Outcome::of(missing.len() + extra.len()))
}
