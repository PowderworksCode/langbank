//! The registries that read best as one sorted table: ecosystems, toolchains,
//! package registries, tool profiles, and the gaps.

use crate::render::{code, codes, link, page};
use maud::{Markup, html};

/// An em dash rather than an empty cell: "nothing here" and "nothing to say"
/// look the same in a table otherwise.
fn or_dash(markup: Option<Markup>) -> Markup {
    markup.unwrap_or_else(|| html! { span.none { "—" } })
}

fn languages_of(languages: &[&'static langbank::LanguageProfile]) -> Markup {
    if languages.is_empty() {
        return or_dash(None);
    }
    let rest = languages.len().saturating_sub(6);
    html! {
        @for (index, language) in languages.iter().take(6).enumerate() {
            @if index > 0 { ", " }
            (link(&format!("/languages/{}", language.id), language.display_name))
        }
        @if rest > 0 { " " span.none { "+" (rest) } }
    }
}

fn table(headers: &[&str], rows: Markup) -> Markup {
    html! {
        div.scroll { table {
            thead { tr { @for header in headers { th { (header) } } } }
            tbody { (rows) }
        } }
    }
}

pub fn ecosystems() -> String {
    let mut all = langbank::ecosystem_profiles().to_vec();
    all.sort_by_key(|e| e.display_name.to_lowercase());
    let rows = html! {
        @for ecosystem in &all {
            tr id=(ecosystem.id) {
                td { (ecosystem.display_name) }
                td { (languages_of(ecosystem.implied_languages)) }
                td { (or_dash(ecosystem.manifest.map(code))) }
                td { (codes(ecosystem.lockfiles)) }
                td {
                    (or_dash(ecosystem.registry.map(|r|
                        link(&format!("/registries#{}", r.id), r.display_name))))
                }
            }
        }
    };
    let body = html! {
        h1 { (all.len()) " ecosystems" }
        p.lede {
            "A manifest, the lockfiles that pin it, and the registry it resolves
             against. This is what tells a walker that a directory is a project
             rather than a folder of files."
        }
        (table(&["ecosystem", "languages", "manifest", "lockfiles", "registry"], rows))
    };
    page("Ecosystems", &[("/", "langbank")], body)
}

pub fn toolchains() -> String {
    let mut all = langbank::toolchains().to_vec();
    all.sort_by_key(|t| t.display_name.to_lowercase());
    let rows = html! {
        @for toolchain in &all {
            tr id=(toolchain.id) {
                td { (toolchain.display_name) }
                td { (format!("{:?}", toolchain.kind).to_lowercase()) }
                td { (languages_of(toolchain.languages)) }
                td { (codes(toolchain.programs)) }
                td {
                    (or_dash(toolchain.version.as_ref().map(|probe| code(&format!(
                        "{} {}",
                        toolchain.programs.first().unwrap_or(&""),
                        probe.arguments.join(" ")
                    )))))
                }
            }
        }
    };
    let body = html! {
        h1 { (all.len()) " toolchains" }
        p.lede {
            "What builds, tests, formats and lints each language, the programs that
             invoke it, and the command that asks it its version. The probe is data —
             langbank states it and the caller runs it."
        }
        (table(&["toolchain", "kind", "languages", "programs", "version probe"], rows))
    };
    page("Toolchains", &[("/", "langbank")], body)
}

pub fn registries() -> String {
    let mut all = langbank::package_registries().to_vec();
    all.sort_by_key(|r| r.display_name.to_lowercase());
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
        format!("{requirement}, {folding}")
    };
    let rows = html! {
        @for registry in &all {
            tr id=(registry.id) {
                td { (code(&format!("pkg:{}", registry.id))) }
                td { (registry.display_name) }
                td { (spelling(&registry.namespace)) }
                td { (spelling(&registry.name)) }
                td { (or_dash(registry.default_repository.map(code))) }
            }
        }
    };
    let body = html! {
        h1 { (all.len()) " package registries" }
        p.lede {
            "How identity is spelled in each registry: whether a namespace is
             required, what case-folds, and where it resolves by default. Two names
             that differ only in case are the same package in some registries and not
             in others, which is the kind of thing that is only a bug once you have
             shipped it."
        }
        (table(&["purl type", "registry", "namespace", "name", "default repository"], rows))
    };
    page("Package registries", &[("/", "langbank")], body)
}

pub fn tools() -> String {
    let mut all = langbank::tool_profiles().to_vec();
    all.sort_by_key(|t| t.id.to_lowercase());
    let rows = html! {
        @for tool in &all {
            tr id=(tool.id) {
                td { (code(tool.id)) }
                td { (codes(tool.programs)) }
                td { (languages_of(tool.languages)) }
                td { (codes(tool.configuration_files)) }
            }
        }
    };
    let body = html! {
        h1 { (all.len()) " tool profiles" }
        p.lede {
            "The programs a repository actually invokes, and the files that configure
             them. Enough to recognise a tool in a CI log or a lockfile without
             hard-coding its name in the consumer."
        }
        (table(&["tool", "programs", "languages", "configuration"], rows))
    };
    page("Tool profiles", &[("/", "langbank")], body)
}

/// A gap's subject, linked when it names a language langbank carries. Some
/// subjects are extensions rather than languages, which is why this is a lookup
/// and not a link.
fn subject(id: &str) -> Markup {
    match langbank::language_profile(id) {
        Some(language) => link(
            &format!("/languages/{}", language.id),
            language.display_name,
        ),
        None => code(id),
    }
}

pub fn gaps() -> String {
    let mut all = langbank::gaps().to_vec();
    all.sort_by_key(|g| (format!("{:?}", g.reason), g.subject));
    let rows = html! {
        @for gap in &all {
            tr {
                td { (subject(gap.subject)) }
                td { (code(gap.facet)) }
                td { (format!("{:?}", gap.reason)) }
                td { (gap.note) }
            }
        }
    };
    let body = html! {
        h1 { (all.len()) " recorded absences" }
        p.lede {
            "A registry that silently omits what it does not know cannot be told apart
             from one nobody has filled in. These are the things langbank was asked and
             declined to answer, each with the reason it declined."
        }
        p {
            "Sources disagreed; only one source said it and nothing corroborated it; it
             was excluded on purpose; or it is not modelled yet. Only the last is a
             to-do."
        }
        (table(&["subject", "facet", "reason", "note"], rows))
    };
    page("Gaps", &[("/", "langbank")], body)
}
