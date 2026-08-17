// A build script that cannot read its data must fail the build. Silently
// generating an empty registry is worse than not compiling at all — a consumer
// would see a language-free world and no error — so panicking here is the
// correct behaviour, and the crate-wide denials are lifted for this file only.
#![allow(clippy::expect_used, clippy::panic)]

//! Generate the language registry from `data/`.
//!
//! The data is TOML because it is data: reviewable in a diff, editable without
//! a compiler, and generable from upstream sources. What it generates is the
//! same `&'static` tables the profiles were written as by hand, so nothing
//! downstream pays a runtime cost for the move — the statics are identical and
//! the registration is identical.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use toml::Value;

fn strs(value: Option<&Value>) -> String {
    let items = value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(|item| format!("{item:?}"))
                .collect()
        });
    format!("&[{}]", items.join(", "))
}

/// `c-sharp` is a legal language id and not a legal module name, and `4d`
/// starts with a digit, which no identifier may.
fn ident(id: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "box", "break", "const", "continue", "crate", "dyn", "else",
        "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
        "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "self",
        "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe", "unsized",
        "use", "virtual", "where", "while", "yield",
    ];
    let name = id.replace('-', "_");
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else if KEYWORDS.contains(&name.as_str()) {
        // `Move` and `Self` are both languages. A raw identifier would do for
        // the first and is not permitted for the second, so both take a suffix.
        format!("{name}_")
    } else {
        name
    }
}

/// `structured-code` names `crate::STRUCTURED_CODE`.
fn screaming(id: &str) -> String {
    id.replace('-', "_").to_uppercase()
}

fn role(value: &str) -> String {
    let mut chars = value.chars();
    let head = chars.next().map(|c| c.to_ascii_uppercase());
    format!(
        "crate::LanguageRole::{}{}",
        head.unwrap_or('?'),
        chars.as_str()
    )
}

fn comment_tables(out: &mut String, path: &Path) {
    let text = std::fs::read_to_string(path).expect("read comment-syntax.toml");
    let tables: BTreeMap<String, Value> = toml::from_str(&text).expect("parse comment-syntax.toml");
    out.push_str("    pub(crate) mod comment_syntax {\n");
    for (name, table) in &tables {
        let block = table
            .get("block")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |pairs| {
                pairs
                    .iter()
                    .filter_map(Value::as_array)
                    .filter_map(|pair| Some((pair.first()?.as_str()?, pair.get(1)?.as_str()?)))
                    .map(|(open, close)| format!("({open:?}, {close:?})"))
                    .collect()
            });
        let quotes = table
            .get("quotes")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(|q| q.chars().next())
                    .map(|q| format!("{q:?}"))
                    .collect()
            });
        writeln!(
            out,
            "        pub(crate) static {}: crate::CommentSyntax = crate::CommentSyntax {{\n\
             \x20            line: {},\n\
             \x20            block: &[{}],\n\
             \x20            documentation: {},\n\
             \x20            quotes: &[{}],\n\
             \x20            multi_quotes: {},\n\
             \x20        }};",
            screaming(name),
            strs(table.get("line")),
            block.join(", "),
            strs(table.get("documentation")),
            quotes.join(", "),
            strs(table.get("multi-quotes")),
        )
        .expect("write comment table");
    }
    out.push_str("    }\n\n");
}

fn conventions(table: &Value) -> String {
    let mut out = String::from("Some(crate::LanguageConventions {\n");
    match table.get("typecheck") {
        Some(typecheck) => {
            let _ = writeln!(
                out,
                "                typecheck: Some(crate::TypecheckConvention {{ config_files: {} }}),",
                strs(typecheck.get("config-files"))
            );
        }
        None => out.push_str("                typecheck: None,\n"),
    }
    let layout = table
        .get("test-layout")
        .expect("conventions need a test layout");
    let _ = write!(
        out,
        "                test_layout: crate::TestLayoutDefaults {{\n\
         \x20                   source_roots: {},\n\
         \x20                   test_root: {:?},\n\
         \x20                   test_suffixes: {},\n\
         \x20               }},\n",
        strs(layout.get("source-roots")),
        layout
            .get("test-root")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        strs(layout.get("test-suffixes")),
    );
    let rules = table
        .get("inline-test")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array
                .iter()
                .map(|rule| {
                    format!(
                        "crate::InlineTestRule {{ starts_with: {}, contains_any: {}, indicator: {:?} }}",
                        strs(rule.get("starts-with")),
                        strs(rule.get("contains-any")),
                        rule.get("indicator").and_then(Value::as_str).unwrap_or_default(),
                    )
                })
                .collect()
        });
    let _ = write!(
        out,
        "                inline_test: &[{}],\n            }})",
        rules.join(", ")
    );
    out
}

fn facets(out: &mut String, path: &Path) {
    let text = std::fs::read_to_string(path).expect("read facets.toml");
    let facets: BTreeMap<String, Value> = toml::from_str(&text).expect("parse facets.toml");
    out.push_str("pub(crate) mod facets {\n");
    for (id, facet) in &facets {
        writeln!(
            out,
            "    pub static {}: crate::LanguageFacet = crate::LanguageFacet {{\n\
             \x20       id: {id:?},\n\
             \x20       description: {:?},\n\
             \x20   }};\n\
             \x20   crate::registry::submit! {{ crate::LanguageFacetRegistration(&{}) }}",
            screaming(id),
            facet
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            screaming(id),
        )
        .expect("write facet");
    }
    out.push_str("}\n\n");
}

fn artifacts(out: &mut String, path: &Path) {
    let text = std::fs::read_to_string(path).expect("read artifacts.toml");
    let artifacts: BTreeMap<String, Value> = toml::from_str(&text).expect("parse artifacts.toml");
    out.push_str("pub(crate) mod artifacts {\n");
    for (id, artifact) in &artifacts {
        let name = format!("{}_ARTIFACT", screaming(id));
        writeln!(
            out,
            "    pub static {name}: crate::ArtifactProfile = crate::ArtifactProfile {{\n\
             \x20       id: {id:?},\n\
             \x20       display_name: {:?},\n\
             \x20       project_facets: {},\n\
             \x20       package_dependencies: {},\n\
             \x20       package_script_signals: {},\n\
             \x20   }};\n\
             \x20   crate::registry::submit! {{ crate::ArtifactRegistration(&{name}) }}",
            artifact
                .get("display-name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            strs(artifact.get("project-facets")),
            strs(artifact.get("package-dependencies")),
            strs(artifact.get("package-script-signals")),
        )
        .expect("write artifact");
    }
    out.push_str("}\n\n");
}

/// `package-manager` names `EcosystemRole::PackageManager`.
fn camel(value: &str) -> String {
    value
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(head) => format!("{}{}", head.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect()
}

/// Every `.toml` under `directory`, sorted, with any failure fatal.
///
/// This used to be `read_dir(..).filter_map(Result::ok)`, which skipped an
/// entry that could not be read and carried on — so an unreadable data file
/// left the registry a language short and the build green. `gaps` was worse
/// still: it took `unwrap_or_default()` on the directory itself, so an
/// unreadable `data/gaps/` compiled to zero recorded absences, which is the
/// exact "language-free world and no error" this file's header refuses to
/// produce. A build that cannot see its data must not finish.
/// `origin` for a generated entry: `homepage` only when it is somewhere other
/// than the repository, which is the rule `crate::Origin` documents.
fn origin(table: &Value) -> String {
    let field = |key: &str| {
        table
            .get(key)
            .and_then(Value::as_str)
            .map(|value| format!("Some({value:?})"))
            .unwrap_or_else(|| "None".to_string())
    };
    format!(
        "crate::Origin {{ homepage: {}, repository: {} }}",
        field("homepage"),
        field("repository")
    )
}

fn toml_files(directory: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    let mut files: Vec<PathBuf> = entries
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("read an entry of {}: {error}", directory.display()))
                .path()
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    files
}

fn ecosystems(out: &mut String, directory: &Path) {
    let files = toml_files(directory);

    // A profile is emitted for every ecosystem; which of them lib.rs surfaces
    // by name is its own choice, so the unused ones are not a warning.
    out.push_str("#[allow(unused_imports)]\npub(crate) mod ecosystems {\n");
    let mut exports = Vec::new();
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).expect("read ecosystem toml");
        let eco: Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let id = eco.get("id").and_then(Value::as_str).expect("ecosystem id");
        exports.push(id.to_owned());

        let roles = eco
            .get("roles")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|role| format!("crate::EcosystemRole::{}", camel(role)))
                    .collect()
            });
        let languages = eco
            .get("implied-languages")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|language| {
                        format!("&super::super::languages::{}::PROFILE", ident(language))
                    })
                    .collect()
            });
        let manifest = eco.get("manifest").and_then(Value::as_str).map_or_else(
            || "None".to_owned(),
            |manifest| format!("Some({manifest:?})"),
        );
        let pins = eco.get("dependency-pins").map_or_else(
            || "None".to_owned(),
            |pins| {
                format!(
                    "Some(crate::DependencyPinPolicy {{ syntax: crate::DependencyPinSyntax::{}, advisory: {} }})",
                    camel(pins.get("syntax").and_then(Value::as_str).unwrap_or("exact-semver")),
                    pins.get("advisory").and_then(Value::as_bool).unwrap_or(false),
                )
            },
        );

        writeln!(
            out,
            "    pub mod {module} {{\n\
             \x20       pub static PROFILE: crate::EcosystemProfile = crate::EcosystemProfile {{\n\
             \x20           origin: {origin},\n\
             \x20           id: {id:?},\n\
             \x20           display_name: {display:?},\n\
             \x20           roles: &[{roles}],\n\
             \x20           implied_languages: &[{languages}],\n\
             \x20           manifest: {manifest},\n\
             \x20           lockfiles: {lockfiles},\n\
             \x20           selector_files: {selectors},\n\
             \x20           alternate_manifests: {alternates},\n\
             \x20           gitignore_patterns: {gitignore},\n\
             \x20           manifest_selection: crate::ManifestSelection::{selection},\n\
             \x20           dependency_pins: {pins},\n\
             \x20           registry: {registry},\n\
             \x20       }};\n\
             \x20       crate::registry::submit! {{ crate::EcosystemRegistration(&PROFILE) }}\n\
             {traversal}\
             \x20   }}",
            origin = origin(&eco),
            module = ident(id),
            display = eco
                .get("display-name")
                .and_then(Value::as_str)
                .unwrap_or(id),
            roles = roles.join(", "),
            languages = languages.join(", "),
            lockfiles = strs(eco.get("lockfiles")),
            selectors = strs(eco.get("selector-files")),
            alternates = strs(eco.get("alternate-manifests")),
            gitignore = strs(eco.get("gitignore-patterns")),
            selection = camel(
                eco.get("manifest-selection")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
            ),
            traversal = traversal(&eco),
            registry = eco.get("registry").and_then(Value::as_str).map_or_else(
                || "None".to_owned(),
                |id| format!("Some(&super::super::registries::{})", screaming(id)),
            ),
        )
        .expect("write ecosystem");
    }
    for id in &exports {
        writeln!(
            out,
            "    pub use {}::PROFILE as {};",
            ident(id),
            screaming(id)
        )
        .expect("write ecosystem export");
    }
    out.push_str("}\n\n");
}

/// Directories an ecosystem generates, declared alongside the ecosystem that
/// generates them so the two cannot drift apart.
fn traversal(eco: &Value) -> String {
    let mut out = String::new();
    let Some(directories) = eco.get("traversal").and_then(Value::as_array) else {
        return out;
    };
    for (index, directory) in directories.iter().enumerate() {
        let _ = write!(
            out,
            "        static TRAVERSAL_{index}: crate::TraversalDirectory = crate::TraversalDirectory {{\n\
             \x20           name: {:?},\n\
             \x20           markers: {},\n\
             \x20       }};\n\
             \x20       crate::registry::submit! {{ crate::TraversalDirectoryRegistration(&TRAVERSAL_{index}) }}\n",
            directory
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            strs(directory.get("markers")),
        );
    }
    out
}

/// `{ exact = "--compile" }` / `{ prefix = "--compile=" }`
fn argument_patterns(value: Option<&Value>) -> String {
    let items = value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array
                .iter()
                .filter_map(|entry| {
                    let table = entry.as_table()?;
                    let (kind, argument) = table.iter().next()?;
                    Some(format!(
                        "crate::ArgumentPattern::{}({:?})",
                        camel(kind),
                        argument.as_str()?
                    ))
                })
                .collect()
        });
    format!("&[{}]", items.join(", "))
}

fn retry_signals(value: Option<&Value>) -> String {
    let items = value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array
                .iter()
                .filter_map(|entry| {
                    let table = entry.as_table()?;
                    let (kind, name) = table.iter().next()?;
                    Some(format!(
                        "crate::TestRetrySignal::{}({:?})",
                        camel(kind),
                        name.as_str()?
                    ))
                })
                .collect()
        });
    format!("&[{}]", items.join(", "))
}

fn commands(tool: &Value) -> String {
    let mut out = String::new();
    let Some(commands) = tool.get("commands").and_then(Value::as_array) else {
        return out;
    };
    for command in commands {
        let tasks = command
            .get("tasks")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|task| format!("crate::TaskKind::{}", camel(task)))
                    .collect()
            });
        let artifacts = command
            .get("artifacts")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|id| format!("&super::artifacts::{}_ARTIFACT", screaming(id)))
                    .collect()
            });
        let _ = write!(
            out,
            "            crate::CommandPattern {{\n\
             \x20               arguments: {},\n\
             \x20               required_arguments: {},\n\
             \x20               rejected_arguments: {},\n\
             \x20               tasks: &[{}],\n\
             \x20               artifacts: &[{}],\n\
             \x20           }},\n",
            strs(command.get("arguments")),
            argument_patterns(command.get("required-arguments")),
            argument_patterns(command.get("rejected-arguments")),
            tasks.join(", "),
            artifacts.join(", "),
        );
    }
    out
}

fn test_retry(tool: &Value) -> String {
    let Some(retry) = tool.get("test-retry") else {
        return "None".to_owned();
    };
    let configurations = retry
        .get("configurations")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array
                .iter()
                .map(|configuration| {
                    format!(
                        "crate::TestRetryConfiguration {{ paths: {}, signals: {} }}",
                        strs(configuration.get("paths")),
                        retry_signals(configuration.get("signals")),
                    )
                })
                .collect()
        });
    format!(
        "Some(crate::TestRetryProfile {{ arguments: {}, configurations: &[{}] }})",
        strs(retry.get("arguments")),
        configurations.join(", "),
    )
}

fn tools(out: &mut String, directory: &Path) {
    let files = toml_files(directory);

    out.push_str("pub(crate) mod tools {\n");
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).expect("read tool toml");
        let tool: Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let id = tool.get("id").and_then(Value::as_str).expect("tool id");
        let languages =
            tool.get("languages")
                .and_then(Value::as_array)
                .map_or_else(Vec::new, |array| {
                    array
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|language| format!("&super::languages::{}::PROFILE", ident(language)))
                        .collect()
                });
        writeln!(
            out,
            "    pub static {name}: crate::ToolProfile = crate::ToolProfile {{\n\
             \x20       origin: {origin},\n\
             \x20       id: {id:?},\n\
             \x20       programs: {programs},\n\
             \x20       languages: &[{languages}],\n\
             \x20       commands: &[\n{commands}\x20       ],\n\
             \x20       configuration_files: {configuration},\n\
             \x20       package_json_keys: {keys},\n\
             \x20       ci_workload: crate::CiWorkload::{workload},\n\
             \x20       test_retry: {retry},\n\
             \x20   }};\n\
             \x20   crate::registry::submit! {{ crate::ToolRegistration(&{name}) }}",
            origin = origin(&tool),
            name = screaming(id),
            programs = strs(tool.get("programs")),
            languages = languages.join(", "),
            commands = commands(&tool),
            configuration = strs(tool.get("configuration-files")),
            keys = strs(tool.get("package-json-keys")),
            workload = camel(
                tool.get("ci-workload")
                    .and_then(Value::as_str)
                    .unwrap_or("light")
            ),
            retry = test_retry(&tool),
        )
        .expect("write tool");
    }
    out.push_str("}\n\n");
}

/// Package registries, as purl defines them. A registry is where a package
/// identity lives; the manager that reads a lockfile is a different thing and
/// lives in data/ecosystems/.
fn registries(out: &mut String, directory: &Path) {
    let files = toml_files(directory);

    out.push_str("pub(crate) mod registries {\n");
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).expect("read registry toml");
        let entry: Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .expect("registry id");
        let component = |key: &str| {
            let part = entry.get(key);
            format!(
                "crate::IdentityComponent {{ requirement: crate::Requirement::{}, case_sensitive: {} }}",
                camel(
                    part.and_then(|part| part.get("requirement"))
                        .and_then(Value::as_str)
                        .unwrap_or("optional")
                ),
                part.and_then(|part| part.get("case-sensitive"))
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            )
        };
        writeln!(
            out,
            "    pub static {name}: crate::PackageRegistry = crate::PackageRegistry {{\n\
             \x20       origin: {origin},\n\
             \x20       id: {id:?},\n\
             \x20       display_name: {display:?},\n\
             \x20       default_repository: {repository},\n\
             \x20       uses_repository: {uses},\n\
             \x20       namespace: {namespace},\n\
             \x20       name: {name_component},\n\
             \x20       version: {version},\n\
             \x20   }};\n\
             \x20   crate::registry::submit! {{ crate::PackageRegistryRegistration(&{name}) }}",
            origin = origin(&entry),
            name = screaming(id),
            display = entry
                .get("display-name")
                .and_then(Value::as_str)
                .unwrap_or(id),
            repository = entry
                .get("default-repository")
                .and_then(Value::as_str)
                .map_or_else(|| "None".to_owned(), |url| format!("Some({url:?})")),
            uses = entry
                .get("uses-repository")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            namespace = component("namespace"),
            name_component = component("name"),
            version = component("version"),
        )
        .expect("write registry");
    }
    out.push_str("}\n\n");
}

/// The programs a language is processed by. Not tool profiles: those classify
/// an invocation, these describe the program itself.
fn toolchains(out: &mut String, directory: &Path) {
    let files = toml_files(directory);

    out.push_str("pub(crate) mod toolchains {\n");
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).expect("read toolchain toml");
        let entry: Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .expect("toolchain id");
        let languages = entry
            .get("languages")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|language| format!("&super::languages::{}::PROFILE", ident(language)))
                    .collect()
            });
        let stream = |table: Option<&Value>| {
            format!(
                "crate::OutputStream::{}",
                camel(
                    table
                        .and_then(|table| table.get("stream"))
                        .and_then(Value::as_str)
                        .unwrap_or("stdout")
                )
            )
        };
        let version = entry.get("version").map_or_else(
            || "None".to_owned(),
            |probe| {
                format!(
                    "Some(crate::VersionProbe {{ arguments: {}, stream: {}, pattern: {:?} }})",
                    strs(probe.get("arguments")),
                    stream(Some(probe)),
                    probe
                        .get("pattern")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
            },
        );
        let diagnostics = entry.get("diagnostics").map_or_else(
            || "None".to_owned(),
            |diag| {
                format!(
                    "Some(crate::DiagnosticFormat {{ format: {:?}, arguments: {}, stream: {} }})",
                    diag.get("format").and_then(Value::as_str).unwrap_or("text"),
                    strs(diag.get("arguments")),
                    stream(Some(diag)),
                )
            },
        );
        writeln!(
            out,
            "    pub static {name}: crate::Toolchain = crate::Toolchain {{\n\
             \x20       origin: {origin},\n\
             \x20       id: {id:?},\n\
             \x20       display_name: {display:?},\n\
             \x20       kind: crate::ToolchainKind::{kind},\n\
             \x20       languages: &[{languages}],\n\
             \x20       programs: {programs},\n\
             \x20       version: {version},\n\
             \x20       diagnostics: {diagnostics},\n\
             \x20       categories: &[{categories}],\n\
             \x20       distribution: {distribution},\n\
             \x20       root_markers: {markers},\n\
             \x20   }};\n\
             \x20   crate::registry::submit! {{ crate::ToolchainRegistration(&{name}) }}",
            origin = origin(&entry),
            name = screaming(id),
            display = entry
                .get("display-name")
                .and_then(Value::as_str)
                .unwrap_or(id),
            kind = camel(
                entry
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("compiler")
            ),
            languages = languages.join(", "),
            programs = strs(entry.get("programs")),
            markers = strs(entry.get("root-markers")),
            categories = entry
                .get("categories")
                .and_then(Value::as_array)
                .map_or_else(Vec::new, |array| {
                    array
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|role| format!("crate::ToolchainKind::{}", camel(role)))
                        .collect()
                })
                .join(", "),
            distribution = entry.get("distribution").map_or_else(
                || "None".to_owned(),
                |dist| {
                    format!(
                        "Some(crate::Distribution {{ registry: {:?}, package: {:?} }})",
                        dist.get("registry")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        dist.get("package")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    )
                },
            ),
        )
        .expect("write toolchain");
    }
    out.push_str("}\n\n");
}

/// Gaps: what langbank knows it does not know, and why.
fn gaps(out: &mut String, directory: &Path) {
    let files = toml_files(directory);

    out.push_str("pub(crate) mod gaps {\n");
    let mut index = 0;
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).expect("read gap toml");
        let table: Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let facet = table
            .get("facet")
            .and_then(Value::as_str)
            .expect("a gap file names the facet it is about");
        let Some(entries) = table.get("gap").and_then(Value::as_array) else {
            continue;
        };
        for entry in entries {
            writeln!(
                out,
                "    pub static GAP_{index}: crate::Gap = crate::Gap {{\n\
                 \x20       subject: {subject:?},\n\
                 \x20       facet: {facet:?},\n\
                 \x20       reason: crate::GapReason::{reason},\n\
                 \x20       note: {note:?},\n\
                 \x20   }};\n\
                 \x20   crate::registry::submit! {{ crate::GapRegistration(&GAP_{index}) }}",
                subject = entry
                    .get("subject")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                reason = camel(
                    entry
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("not-modelled")
                ),
                note = entry
                    .get("note")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
            .expect("write gap");
            index += 1;
        }
    }
    out.push_str("}\n\n");
}

/// Content rules for extensions several languages claim.
/// One rule's clauses, as constructor source. Every clause must hold, so an
/// empty list is a rule that always matches — which is how the fallback at the
/// end of a block is spelled.
fn disambiguation_clauses(rule: &Value) -> Vec<String> {
    rule.get("clause")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |clauses| {
            clauses
                .iter()
                .map(|clause| {
                    format!(
                        "crate::Clause {{ patterns: {}, negative: {} }}",
                        strs(clause.get("patterns")),
                        strs(clause.get("negative")),
                    )
                })
                .collect()
        })
}

/// One block's rules, in the order linguist evaluates them. The order is the
/// meaning here — the first rule whose clauses all match wins — so this must
/// not sort or dedupe.
fn disambiguation_rules(block: &Value) -> Vec<String> {
    block
        .get("rule")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array
                .iter()
                .map(|rule| {
                    format!(
                        "crate::DisambiguationRule {{ language: &super::languages::{}::PROFILE, clauses: &[{}], portable: {} }}",
                        ident(rule.get("language").and_then(Value::as_str).unwrap_or_default()),
                        disambiguation_clauses(rule).join(", "),
                        rule.get("portable").and_then(Value::as_bool).unwrap_or(true),
                    )
                })
                .collect()
        })
}

fn disambiguations(out: &mut String, path: &Path) {
    if !path.exists() {
        return;
    }
    println!("cargo:rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(path).expect("read heuristics.toml");
    let document: Value = toml::from_str(&text).expect("parse heuristics.toml");
    let Some(blocks) = document.get("disambiguation").and_then(Value::as_array) else {
        return;
    };

    out.push_str("pub(crate) mod heuristics {\n");
    for (index, block) in blocks.iter().enumerate() {
        let rules = disambiguation_rules(block);
        writeln!(
            out,
            "    pub static BLOCK_{index}: crate::Disambiguation = crate::Disambiguation {{\n\
             \x20       extensions: {},\n\
             \x20       rules: &[{}],\n\
             \x20   }};\n\
             \x20   crate::registry::submit! {{ crate::DisambiguationRegistration(&BLOCK_{index}) }}",
            strs(block.get("extensions")),
            rules.join(", "),
        )
        .expect("write disambiguation");
    }
    out.push_str("}\n\n");
}

fn main() {
    let data = Path::new("data");
    println!("cargo:rerun-if-changed=data");

    let mut out = String::from("// @generated by build.rs from data/. Do not edit.\n");
    registries(&mut out, &data.join("registries"));
    facets(&mut out, &data.join("facets.toml"));
    artifacts(&mut out, &data.join("artifacts.toml"));
    ecosystems(&mut out, &data.join("ecosystems"));
    tools(&mut out, &data.join("tools"));

    out.push_str("pub(crate) mod languages {\n");
    comment_tables(&mut out, &data.join("comment-syntax.toml"));

    let files = toml_files(&data.join("languages"));

    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).expect("read language toml");
        let profile: Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let id = profile.get("id").and_then(Value::as_str).expect("id");

        let facets =
            profile
                .get("facets")
                .and_then(Value::as_array)
                .map_or_else(Vec::new, |array| {
                    array
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|facet| format!("&super::super::facets::{}", screaming(facet)))
                        .collect()
                });
        let groups_under = profile
            .get("groups-under")
            .and_then(Value::as_str)
            .map_or_else(
                || "None".to_string(),
                |other| format!("Some(&super::{}::PROFILE)", ident(other)),
            );
        let supersedes = profile
            .get("supersedes")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|other| format!("&super::{}::PROFILE", ident(other)))
                    .collect()
            });
        let comments = profile.get("comments").and_then(Value::as_str).map_or_else(
            || "None".to_owned(),
            |name| format!("Some(&super::comment_syntax::{})", screaming(name)),
        );
        // A language that does not state source extensions accepts exactly what
        // identifies it, which is what the hand-written profiles did.
        let source = profile
            .get("source-extensions")
            .or_else(|| profile.get("extensions"));

        writeln!(
            out,
            "    pub mod {module} {{\n\
             \x20       pub static PROFILE: crate::LanguageProfile = crate::LanguageProfile {{\n\
             \x20           id: {id:?},\n\
             \x20           display_name: {name:?},\n\
             \x20           extensions: {extensions},\n\
             \x20           source_extensions: {source},\n\
             \x20           filenames: {filenames},\n\
             \x20           shebangs: {shebangs},\n\
             \x20           role: {role},\n\
             \x20           facets: &[{facets}],\n\
             \x20           comments: {comments},\n\
             \x20           conventions: {conventions},\n\
             \x20           config_files: {config},\n\
             \x20           package_dependencies: {deps},\n\
             \x20           supersedes: &[{supersedes}],\n\
             \x20           groups_under: {groups_under},\n\
             \x20           primary_extensions: {primary},\n\
             \x20       }};\n\
             \x20       crate::registry::submit! {{ crate::LanguageRegistration(&PROFILE) }}\n\
             \x20   }}",
            module = ident(id),
            name = profile
                .get("display-name")
                .and_then(Value::as_str)
                .unwrap_or(id),
            extensions = strs(profile.get("extensions")),
            source = strs(source),
            filenames = strs(profile.get("filenames")),
            shebangs = strs(profile.get("shebangs")),
            role = role(
                profile
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("programming")
            ),
            facets = facets.join(", "),
            comments = comments,
            conventions = profile
                .get("conventions")
                .map_or_else(|| "None".to_owned(), conventions),
            config = strs(profile.get("config-files")),
            deps = strs(profile.get("package-dependencies")),
            supersedes = supersedes.join(", "),
            groups_under = groups_under,
            primary = strs(profile.get("primary-extensions")),
        )
        .expect("write profile");
    }
    out.push_str("}\n");
    toolchains(&mut out, &data.join("toolchains"));
    gaps(&mut out, &data.join("gaps"));
    disambiguations(&mut out, &data.join("heuristics.toml"));

    let destination = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("registries.rs");
    std::fs::write(&destination, out).expect("write generated registries");
}
