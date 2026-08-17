//! One page per thing: a toolchain, an ecosystem, a registry, a tool.
//!
//! Everything langbank carries is addressable now — `/toolchains/bun`,
//! `/registries/npm` — rather than an anchor into a table of a thousand rows.
//! The tables stay as an index and link into these.

use crate::render::{code, codes, link, page, row};
use langbank::Origin;
use maud::{Markup, html};

/// The links a thing has, as a line under its title.
///
/// `Origin` carries a homepage only when it is somewhere other than the
/// repository, so a project whose site is its README shows one link rather than
/// the same URL twice.
fn origin(origin: &Origin) -> Markup {
    html! {
        @if origin.is_unknown() {
            p.dim.links { "No website or source recorded." }
        } @else {
            p.links {
                @for (label, url) in origin.links() {
                    a.ext href=(url) rel="noopener" { (label) }
                }
            }
        }
    }
}

fn some(condition: bool, markup: Markup) -> Option<Markup> {
    condition.then_some(markup)
}

fn language_links(languages: &[&'static langbank::LanguageProfile]) -> Markup {
    html! {
        @for (index, language) in languages.iter().enumerate() {
            @if index > 0 { ", " }
            (link(&format!("/languages/{}", language.id), language.display_name))
        }
    }
}

pub fn toolchain(id: &str) -> Option<String> {
    let entry = langbank::toolchain(id)?;
    let body = html! {
        h1 { (entry.display_name) }
        (origin(&entry.origin))
        dl {
            (row("id", Some(code(entry.id))))
            // Primary role first, then anything else it does. `categories`
            // holds only the others, so this is the whole picture in one row.
            (row("does", Some(html! {
                @for (index, role) in entry.roles().enumerate() {
                    @if index > 0 { ", " }
                    (format!("{role:?}").to_lowercase())
                }
            })))
            (row("programs", some(!entry.programs.is_empty(), codes(entry.programs))))
            (row("languages", some(!entry.languages.is_empty(), language_links(entry.languages))))
            (row("root markers", some(!entry.root_markers.is_empty(), codes(entry.root_markers))))
            (row("version probe", entry.version.as_ref().map(|probe| html! {
                (code(&format!("{} {}", entry.programs.first().unwrap_or(&""),
                               probe.arguments.join(" "))))
            })))
            (row("diagnostics", entry.diagnostics.as_ref().map(|format| html! {
                (format!("{:?}", format.format).to_lowercase())
                " on " (format!("{:?}", format.stream).to_lowercase())
            })))
            (row("distribution", entry.distribution.as_ref().map(|dist| html! {
                (link(&format!("/registries/{}", dist.registry), dist.registry))
                " " (code(dist.package))
            })))
        }
        p { (link("/toolchains", "← every toolchain")) }
    };
    Some(page(
        entry.display_name,
        &[("/", "langbank"), ("/toolchains", "toolchains")],
        body,
    ))
}

pub fn ecosystem(id: &str) -> Option<String> {
    let entry = langbank::ecosystem_profile(id)?;
    let body = html! {
        h1 { (entry.display_name) }
        (origin(&entry.origin))
        dl {
            (row("id", Some(code(entry.id))))
            (row("roles", some(!entry.roles.is_empty(), html! {
                @for (index, role) in entry.roles.iter().enumerate() {
                    @if index > 0 { ", " }
                    (format!("{role:?}").to_lowercase())
                }
            })))
            (row("languages", some(!entry.implied_languages.is_empty(),
                language_links(entry.implied_languages))))
            (row("manifest", entry.manifest.map(code)))
            (row("lockfiles", some(!entry.lockfiles.is_empty(), codes(entry.lockfiles))))
            (row("selector files", some(!entry.selector_files.is_empty(),
                codes(entry.selector_files))))
            (row("also accepts", some(!entry.alternate_manifests.is_empty(),
                codes(entry.alternate_manifests))))
            (row("ignores", some(!entry.gitignore_patterns.is_empty(),
                codes(entry.gitignore_patterns))))
            (row("registry", entry.registry.map(|registry| html! {
                (link(&format!("/registries/{}", registry.id), registry.display_name))
            })))
            (row("pins", entry.dependency_pins.as_ref().map(|pins| html! {
                (format!("{:?}", pins.syntax).to_lowercase())
                @if pins.advisory { ", advisory" }
            })))
        }
        p { (link("/ecosystems", "← every ecosystem")) }
    };
    Some(page(
        entry.display_name,
        &[("/", "langbank"), ("/ecosystems", "ecosystems")],
        body,
    ))
}

pub fn registry(id: &str) -> Option<String> {
    let entry = langbank::package_registry(id)?;
    let spelling = |component: &langbank::IdentityComponent| {
        let requirement = match component.requirement {
            langbank::Requirement::Required => "required",
            langbank::Requirement::Optional => "optional",
            langbank::Requirement::Prohibited => "prohibited",
        };
        let folding = if component.case_sensitive {
            "case-sensitive"
        } else {
            "case-folded"
        };
        html! { (requirement) ", " (folding) }
    };
    let ecosystems: Vec<&'static langbank::EcosystemProfile> = langbank::ecosystem_profiles()
        .iter()
        .filter(|e| e.registry.is_some_and(|r| r.id == entry.id))
        .copied()
        .collect();

    let body = html! {
        h1 { (entry.display_name) }
        (origin(&entry.origin))
        dl {
            (row("purl type", Some(code(&format!("pkg:{}", entry.id)))))
            (row("namespace", Some(spelling(&entry.namespace))))
            (row("name", Some(spelling(&entry.name))))
            (row("version", Some(spelling(&entry.version))))
            (row("default repository", entry.default_repository.map(code)))
            (row("uses a repository", Some(html! {
                @if entry.uses_repository { "yes" } @else { "no" }
            })))
            (row("published by", some(!ecosystems.is_empty(), html! {
                @for (index, ecosystem) in ecosystems.iter().enumerate() {
                    @if index > 0 { ", " }
                    (link(&format!("/ecosystems/{}", ecosystem.id), ecosystem.display_name))
                }
            })))
        }
        p { (link("/registries", "← every registry")) }
    };
    Some(page(
        entry.display_name,
        &[("/", "langbank"), ("/registries", "registries")],
        body,
    ))
}

pub fn tool(id: &str) -> Option<String> {
    let entry = langbank::tool_profile(id)?;
    // A profile covering several programs is a category, not a project:
    // `javascript-formatter` is prettier and dprint, and a single homepage
    // would be a claim about one of them. Saying so is different from
    // reporting a gap.
    let is_category = entry.programs.len() > 1;
    let body = html! {
        h1 { (entry.id) }
        @if is_category && entry.origin.is_unknown() {
            p.dim.links {
                "A category rather than one project — it covers "
                (entry.programs.len()) " programs, which have their own homes."
            }
        } @else {
            (origin(&entry.origin))
        }
        dl {
            (row("programs", some(!entry.programs.is_empty(), codes(entry.programs))))
            (row("languages", some(!entry.languages.is_empty(), language_links(entry.languages))))
            (row("configuration", some(!entry.configuration_files.is_empty(),
                codes(entry.configuration_files))))
            (row("package.json keys", some(!entry.package_json_keys.is_empty(),
                codes(entry.package_json_keys))))
            (row("CI workload", Some(html! {
                (format!("{:?}", entry.ci_workload).to_lowercase())
            })))
            (row("invocations", some(!entry.commands.is_empty(), html! {
                @for command in entry.commands {
                    @if !command.arguments.is_empty() {
                        div { (code(&format!(
                            "{} {}",
                            entry.programs.first().unwrap_or(&entry.id),
                            command.arguments.join(" ")
                        ))) }
                    }
                }
            })))
        }
        p { (link("/tools", "← every tool profile")) }
    };
    Some(page(
        entry.id,
        &[("/", "langbank"), ("/tools", "tool profiles")],
        body,
    ))
}
