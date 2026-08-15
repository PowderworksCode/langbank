//! The registries that read best as one sorted table: ecosystems, toolchains,
//! package registries, tool profiles, and the gaps.

use crate::render::{code, codes, escape, link, page};
use std::fmt::Write as _;

/// Rows carry an `id` anchor so a language page can link straight at one.
fn table(headers: &[&str], rows: String) -> String {
    let head = headers
        .iter()
        .map(|h| format!("<th>{}", escape(h)))
        .collect::<Vec<_>>()
        .join("");
    format!("<div class=scroll><table><thead><tr>{head}</thead><tbody>{rows}</tbody></table></div>")
}

fn languages_of(languages: &[&'static langbank::LanguageProfile]) -> String {
    if languages.is_empty() {
        return "<span class=none>—</span>".into();
    }
    let shown: Vec<String> = languages
        .iter()
        .take(6)
        .map(|l| link(&format!("/languages/{}", l.id), l.display_name))
        .collect();
    let rest = languages.len().saturating_sub(6);
    if rest > 0 {
        format!("{} <span class=none>+{rest}</span>", shown.join(", "))
    } else {
        shown.join(", ")
    }
}

pub fn ecosystems() -> String {
    let mut all = langbank::ecosystem_profiles().to_vec();
    all.sort_by_key(|e| e.display_name.to_lowercase());
    let mut rows = String::new();
    for e in &all {
        let _ = write!(
            rows,
            "<tr id=\"{}\"><td>{}<td>{}<td>{}<td>{}<td>{}",
            escape(e.id),
            escape(e.display_name),
            languages_of(e.implied_languages),
            e.manifest
                .map(code)
                .unwrap_or_else(|| "<span class=none>—</span>".into()),
            codes(e.lockfiles),
            e.registry
                .map(|r| link(&format!("/registries#{}", r.id), r.display_name))
                .unwrap_or_else(|| "<span class=none>—</span>".into()),
        );
    }
    let body = format!(
        "<h1>{} ecosystems</h1>\n\
         <p class=lede>A manifest, the lockfiles that pin it, and the registry it resolves \
         against. This is what tells a walker that a directory is a project rather than a \
         folder of files.</p>{}",
        all.len(),
        table(
            &[
                "ecosystem",
                "languages",
                "manifest",
                "lockfiles",
                "registry"
            ],
            rows
        )
    );
    page("Ecosystems", &[("/", "langbank")], &body)
}

pub fn toolchains() -> String {
    let mut all = langbank::toolchains().to_vec();
    all.sort_by_key(|t| t.display_name.to_lowercase());
    let mut rows = String::new();
    for t in &all {
        let version = t
            .version
            .as_ref()
            .map(|v| {
                code(&format!(
                    "{} {}",
                    t.programs.first().unwrap_or(&""),
                    v.arguments.join(" ")
                ))
            })
            .unwrap_or_else(|| "<span class=none>—</span>".into());
        let _ = write!(
            rows,
            "<tr id=\"{}\"><td>{}<td>{}<td>{}<td>{}<td>{}",
            escape(t.id),
            escape(t.display_name),
            escape(&format!("{:?}", t.kind).to_lowercase()),
            languages_of(t.languages),
            codes(t.programs),
            version,
        );
    }
    let body = format!(
        "<h1>{} toolchains</h1>\n\
         <p class=lede>What builds, tests, formats and lints each language, the programs that \
         invoke it, and the command that asks it its version. The probe is data — langbank \
         states it and the caller runs it.</p>{}",
        all.len(),
        table(
            &[
                "toolchain",
                "kind",
                "languages",
                "programs",
                "version probe"
            ],
            rows
        )
    );
    page("Toolchains", &[("/", "langbank")], &body)
}

pub fn registries() -> String {
    let mut all = langbank::package_registries().to_vec();
    all.sort_by_key(|r| r.display_name.to_lowercase());
    let mut rows = String::new();
    for r in &all {
        let spelling = |c: &langbank::IdentityComponent| {
            let mut notes = vec![];
            notes.push(match c.requirement {
                langbank::Requirement::Required => "required",
                langbank::Requirement::Optional => "optional",
                langbank::Requirement::Prohibited => "prohibited",
            });
            if c.case_sensitive {
                notes.push("case-sensitive");
            } else {
                notes.push("case-folded");
            }
            escape(&notes.join(", "))
        };
        let _ = write!(
            rows,
            "<tr id=\"{}\"><td>{}<td>{}<td>{}<td>{}<td>{}",
            escape(r.id),
            code(&format!("pkg:{}", r.id)),
            escape(r.display_name),
            spelling(&r.namespace),
            spelling(&r.name),
            r.default_repository
                .map(code)
                .unwrap_or_else(|| "<span class=none>—</span>".into()),
        );
    }
    let body = format!(
        "<h1>{} package registries</h1>\n\
         <p class=lede>How identity is spelled in each registry: whether a namespace is \
         required, what case-folds, and where it resolves by default. Two names that differ \
         only in case are the same package in some registries and not in others, which is the \
         kind of thing that is only a bug once you have shipped it.</p>{}",
        all.len(),
        table(
            &[
                "purl type",
                "registry",
                "namespace",
                "name",
                "default repository"
            ],
            rows
        )
    );
    page("Package registries", &[("/", "langbank")], &body)
}

pub fn tools() -> String {
    let mut all = langbank::tool_profiles().to_vec();
    all.sort_by_key(|t| t.id.to_lowercase());
    let mut rows = String::new();
    for t in &all {
        let _ = write!(
            rows,
            "<tr id=\"{}\"><td>{}<td>{}<td>{}<td>{}",
            escape(t.id),
            code(t.id),
            codes(t.programs),
            languages_of(t.languages),
            codes(t.configuration_files),
        );
    }
    let body = format!(
        "<h1>{} tool profiles</h1>\n\
         <p class=lede>The programs a repository actually invokes, and the files that \
         configure them. Enough to recognise a tool in a CI log or a lockfile without \
         hard-coding its name in the consumer.</p>{}",
        all.len(),
        table(&["tool", "programs", "languages", "configuration"], rows)
    );
    page("Tool profiles", &[("/", "langbank")], &body)
}

pub fn gaps() -> String {
    let mut all = langbank::gaps().to_vec();
    all.sort_by_key(|g| (format!("{:?}", g.reason), g.subject));
    let mut rows = String::new();
    for g in &all {
        let subject = langbank::language_profile(g.subject)
            .map(|l| link(&format!("/languages/{}", l.id), l.display_name))
            .unwrap_or_else(|| code(g.subject));
        let _ = write!(
            rows,
            "<tr><td>{subject}<td>{}<td>{}<td>{}",
            code(g.facet),
            escape(&format!("{:?}", g.reason)),
            escape(g.note),
        );
    }
    let body = format!(
        "<h1>{} recorded absences</h1>\n\
         <p class=lede>A registry that silently omits what it does not know cannot be told \
         apart from one nobody has filled in. These are the things langbank was asked and \
         declined to answer, each with the reason it declined.</p>\n\
         <p>Sources disagreed; only one source said it and nothing corroborated it; it was \
         excluded on purpose; or it is not modelled yet. Only the last is a to-do.</p>{}",
        all.len(),
        table(&["subject", "facet", "reason", "note"], rows)
    );
    page("Gaps", &[("/", "langbank")], &body)
}
