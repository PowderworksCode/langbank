//! Print every registry as one JSON manifest, for the documentation site.
//!
//! The site displays what this prints: `scripts/data-manifest.sh` writes it to
//! `site/content/langbank.json`, the site's build renders reference pages from
//! it, and CI fails when the committed copy no longer matches the binary. The
//! manifest comes out of the compiled tables rather than being maintained
//! beside them, so the website can disagree with the crate only by failing.
//!
//! An example rather than a binary, so the leaf keeps its promise: a consumer
//! takes no exporter and no serde_json — examples compile against
//! dev-dependencies only.

// Stdout is the interface: the script that runs this example redirects it
// into the site's content tree.
#![allow(clippy::print_stdout)]

use langbank::{Facet, Knowledge, LanguageProfile};
use serde_json::{Value, json};

/// Enum variants as the site spells them: `Programming` → `programming`,
/// exactly as langbank.dev renders them.
fn lower(value: impl std::fmt::Debug) -> String {
    format!("{value:?}").to_lowercase()
}

/// Display names, which is how a reader meets a language in a table cell.
fn names(languages: &[&'static LanguageProfile]) -> Vec<&'static str> {
    languages
        .iter()
        .map(|language| language.display_name)
        .collect()
}

fn spelling(component: &langbank::IdentityComponent) -> Value {
    let requirement = match component.requirement {
        langbank::Requirement::Required => "required",
        langbank::Requirement::Optional => "optional",
        langbank::Requirement::Prohibited => "prohibited",
    };
    json!({
        "requirement": requirement,
        "case_sensitive": component.case_sensitive,
    })
}

fn facets() -> Vec<Value> {
    Facet::ALL
        .into_iter()
        .zip(langbank::coverage())
        .map(|(facet, languages)| {
            json!({
                "name": facet.name(),
                "purpose": facet.purpose(),
                "languages": languages,
            })
        })
        .collect()
}

fn languages() -> Vec<Value> {
    let mut all = langbank::language_profiles().to_vec();
    all.sort_by_key(|language| language.display_name.to_lowercase());
    all.iter()
        .map(|language| {
            let knows: Vec<&str> = Knowledge::of(language)
                .facets()
                .filter(|(_, have)| *have)
                .map(|(facet, _)| facet.name())
                .collect();
            json!({
                "id": language.id,
                "name": language.display_name,
                "role": lower(language.role),
                "extensions": language.extensions,
                "primary_extensions": language.primary_extensions,
                "knows": knows,
            })
        })
        .collect()
}

fn ecosystems() -> Vec<Value> {
    let mut all = langbank::ecosystem_profiles().to_vec();
    all.sort_by_key(|ecosystem| ecosystem.display_name.to_lowercase());
    all.iter()
        .map(|ecosystem| {
            json!({
                "id": ecosystem.id,
                "name": ecosystem.display_name,
                "languages": names(ecosystem.implied_languages),
                "manifest": ecosystem.manifest,
                "lockfiles": ecosystem.lockfiles,
                "registry": ecosystem.registry.map(|registry| registry.id),
            })
        })
        .collect()
}

fn toolchains() -> Vec<Value> {
    let mut all = langbank::toolchains().to_vec();
    all.sort_by_key(|toolchain| toolchain.display_name.to_lowercase());
    all.iter()
        .map(|toolchain| {
            let roles: Vec<String> = toolchain.roles().map(lower).collect();
            let probe = toolchain.version.as_ref().map(|probe| {
                let program = toolchain.programs.first().unwrap_or(&"");
                format!("{program} {}", probe.arguments.join(" "))
            });
            json!({
                "id": toolchain.id,
                "name": toolchain.display_name,
                "roles": roles,
                "languages": names(toolchain.languages),
                "programs": toolchain.programs,
                "version_probe": probe,
            })
        })
        .collect()
}

fn registries() -> Vec<Value> {
    let mut all = langbank::package_registries().to_vec();
    all.sort_by_key(|registry| registry.display_name.to_lowercase());
    all.iter()
        .map(|registry| {
            json!({
                "id": registry.id,
                "name": registry.display_name,
                "namespace": spelling(&registry.namespace),
                "package_name": spelling(&registry.name),
                "version": spelling(&registry.version),
                "repository": registry.default_repository,
            })
        })
        .collect()
}

fn tools() -> Vec<Value> {
    let mut all = langbank::tool_profiles().to_vec();
    all.sort_by_key(|tool| tool.id.to_lowercase());
    all.iter()
        .map(|tool| {
            json!({
                "id": tool.id,
                "programs": tool.programs,
                "languages": names(tool.languages),
                "configuration": tool.configuration_files,
            })
        })
        .collect()
}

fn gaps() -> Vec<Value> {
    let mut all = langbank::gaps().to_vec();
    all.sort_by_key(|gap| (format!("{:?}", gap.reason), gap.subject));
    all.iter()
        .map(|gap| {
            json!({
                "subject": gap.subject,
                "facet": gap.facet,
                "reason": lower(gap.reason),
                "note": gap.note,
            })
        })
        .collect()
}

fn main() -> Result<(), serde_json::Error> {
    let languages = languages();
    let ecosystems = ecosystems();
    let toolchains = toolchains();
    let registries = registries();
    let tools = tools();
    let gaps = gaps();
    let manifest = json!({
        "schema": "langbank.data/1",
        "version": env!("CARGO_PKG_VERSION"),
        "counts": {
            "languages": languages.len(),
            "ecosystems": ecosystems.len(),
            "toolchains": toolchains.len(),
            "registries": registries.len(),
            "tools": tools.len(),
            "gaps": gaps.len(),
        },
        "facets": facets(),
        "languages": languages,
        "ecosystems": ecosystems,
        "toolchains": toolchains,
        "registries": registries,
        "tools": tools,
        "gaps": gaps,
    });
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
