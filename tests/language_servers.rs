//! Language servers, absorbed from nvim-lspconfig as toolchain entries.
//!
//! The point of interest is where root markers live. They are recorded on the
//! server, not on the language, because they are the server's convention.
//! Aggregating them per language was tried and produces noise: most servers
//! that list `rust` among their filetypes are generic formatters and
//! spellcheckers whose markers are their own config files, and in that pile
//! `Cargo.toml` is outvoted by `dprint.json`.

use langbank::*;

fn servers() -> impl Iterator<Item = &'static Toolchain> {
    toolchains()
        .iter()
        .copied()
        .filter(|entry| entry.kind == ToolchainKind::LanguageServer)
}

#[test]
fn language_servers_landed_as_toolchains() {
    assert!(servers().count() > 200, "{} servers", servers().count());
    // and they did not displace the hand-written compiler entries
    assert_eq!(
        toolchain("rustc").map(|t| t.kind),
        Some(ToolchainKind::Compiler)
    );
    assert_eq!(
        toolchain("gcc").map(|t| t.kind),
        Some(ToolchainKind::Compiler)
    );
}

#[test]
fn a_server_names_its_program_and_the_languages_it_serves() {
    let clangd = toolchain("lsp-clangd").expect("clangd");
    assert_eq!(clangd.programs, &["clangd"]);
    for id in ["c", "cpp"] {
        let language = language_profile(id).unwrap_or_else(|| panic!("{id}"));
        assert!(clangd.handles(language), "clangd serves {id}");
    }
}

#[test]
fn root_markers_are_the_servers_own_convention() {
    let clangd = toolchain("lsp-clangd").expect("clangd");
    assert!(clangd.root_markers.contains(&"compile_commands.json"));
    assert!(clangd.root_markers.contains(&".clangd"));

    // deno decides a TypeScript project by its own lockfile and config, which
    // is a different convention from clangd's and belongs to deno rather than
    // to TypeScript. Two servers for one language disagreeing about where a
    // project starts is exactly why this is not a language-level fact.
    let deno = toolchain("lsp-denols").expect("denols");
    assert_eq!(deno.root_markers, &["deno.lock", "deno.json", "deno.jsonc"]);

    let pyright = toolchain("lsp-pyright").expect("pyright");
    assert!(pyright.root_markers.contains(&"pyrightconfig.json"));
    assert!(pyright.root_markers.contains(&"pyproject.toml"));
}

#[test]
fn a_server_that_computes_its_root_in_code_carries_no_markers() {
    // rust_analyzer, gopls and jdtls all use `root_dir = function`, which is
    // not data and is not guessed at. They are still carried, for identity.
    let rust_analyzer = toolchain("lsp-rust-analyzer").expect("rust_analyzer");
    assert!(rust_analyzer.root_markers.is_empty());
    assert_eq!(rust_analyzer.programs, &["rust-analyzer"]);
    assert!(rust_analyzer.handles(language_profile("rust").expect("rust")));
}

#[test]
fn a_language_can_be_asked_which_servers_serve_it() {
    let python = language_profile("python").expect("python");
    let names = toolchains_for(python)
        .iter()
        .filter(|t| t.kind == ToolchainKind::LanguageServer)
        .count();
    assert!(names > 5, "python has {names} servers");

    // and the compiler entries are still reachable the same way
    let rust = language_profile("rust").expect("rust");
    let kinds = toolchains_for(rust)
        .iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&ToolchainKind::Compiler));
    assert!(kinds.contains(&ToolchainKind::LanguageServer));
}

#[test]
fn generic_tooling_was_not_absorbed_as_a_language_server() {
    // dprint, typos and ast-grep list ten or more filetypes. Their markers are
    // their own config files and describe no language, so they are excluded.
    for id in [
        "lsp-dprint",
        "lsp-typos-lsp",
        "lsp-ast-grep",
        "lsp-codebook",
    ] {
        assert!(
            toolchain(id).is_none(),
            "{id} should have been excluded as generic"
        );
    }
}

#[test]
fn every_server_has_a_program_and_at_least_one_language() {
    for server in servers() {
        assert!(
            !server.programs.is_empty(),
            "{} names no program",
            server.id
        );
        assert!(
            !server.languages.is_empty(),
            "{} serves no language",
            server.id
        );
        assert!(
            server.version.is_none(),
            "{}: servers carry no version probe",
            server.id
        );
    }
}
