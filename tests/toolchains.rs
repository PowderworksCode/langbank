//! Toolchains: which program implements a language, and how to interrogate it.
//!
//! Every version probe in `data/toolchains/` was run against a real installed
//! program before it was written down — 14 of the 16 on the machine this was
//! built on. `tools/verify-toolchains.py` re-runs them anywhere.

use langbank::*;

#[test]
fn a_toolchain_names_the_languages_it_processes() {
    let rustc = toolchain("rustc").expect("rustc");
    assert_eq!(rustc.kind, ToolchainKind::Compiler);
    let rust = language_profile("rust").expect("rust");
    assert!(rustc.handles(rust));
    assert!(!rustc.handles(language_profile("go").expect("go")));

    // and the inverse question, which is the one a consumer usually asks
    let for_rust = toolchains_for(rust)
        .iter()
        .map(|t| t.id)
        .collect::<Vec<_>>();
    assert!(for_rust.contains(&"rustc"));
    assert!(for_rust.contains(&"cargo"));
}

#[test]
fn one_toolchain_can_serve_several_languages() {
    let gcc = toolchain("gcc").expect("gcc");
    assert!(gcc.handles(language_profile("c").expect("c")));
    assert!(gcc.handles(language_profile("cpp").expect("cpp")));

    let node = toolchain("node").expect("node");
    assert!(node.handles(language_profile("typescript").expect("typescript")));
    assert_eq!(node.kind, ToolchainKind::Runtime);
}

#[test]
fn version_probes_state_the_stream_because_it_differs() {
    // The fact this exists for: `java -version` writes to stderr while
    // `javac -version` writes to stdout. Same vendor, same flag spelling.
    let java = toolchain("java")
        .and_then(|t| t.version)
        .expect("java probe");
    let javac = toolchain("javac")
        .and_then(|t| t.version)
        .expect("javac probe");
    assert_eq!(java.arguments, &["-version"]);
    assert_eq!(javac.arguments, &["-version"]);
    assert_eq!(java.stream, OutputStream::Stderr);
    assert_eq!(javac.stream, OutputStream::Stdout);
}

#[test]
fn a_program_list_is_a_fallback_chain_not_a_single_name() {
    // The unversioned `clang` is absent on plenty of machines that have clang;
    // packaged builds land as `clang-21`. A consumer probing only the first
    // entry decides there is no C compiler when there are two.
    let clang = toolchain("clang").expect("clang");
    assert!(clang.programs.len() > 1, "{:?}", clang.programs);
    assert_eq!(clang.programs.first(), Some(&"clang"));
    assert!(clang.programs.iter().any(|p| p.starts_with("clang-")));

    // python is the same story for a different reason: `python` is Python 2 on
    // older systems and missing on newer ones.
    let python = toolchain("python").expect("python");
    assert_eq!(python.programs.first(), Some(&"python3"));
}

#[test]
fn machine_readable_diagnostics_are_recorded_where_they_exist() {
    let rustc = toolchain("rustc")
        .and_then(|t| t.diagnostics)
        .expect("rustc diagnostics");
    assert_eq!(rustc.format, "json");
    assert_eq!(rustc.arguments, &["--error-format=json"]);
    assert_eq!(rustc.stream, OutputStream::Stderr);

    let gcc = toolchain("gcc")
        .and_then(|t| t.diagnostics)
        .expect("gcc diagnostics");
    assert_eq!(gcc.arguments, &["-fdiagnostics-format=json"]);

    // absent where the tool has none, rather than invented
    assert!(toolchain("node").and_then(|t| t.diagnostics).is_none());
}

#[test]
fn every_toolchain_probe_is_well_formed() {
    for entry in toolchains() {
        assert!(!entry.programs.is_empty(), "{} names no program", entry.id);
        if let Some(probe) = entry.version {
            assert!(
                !probe.arguments.is_empty(),
                "{} probes with no arguments",
                entry.id
            );
            assert!(
                probe.pattern.contains('('),
                "{} has no capture group in {:?}",
                entry.id,
                probe.pattern
            );
        }
    }
}

#[test]
fn no_two_toolchains_describe_the_same_tool() {
    // One tool arriving from two sources used to become two entries — mason's
    // `basedpyright` beside lspconfig's — and which one a sync tool matched
    // depended on the order the filesystem happened to return files in. That
    // made a CI check pass locally and fail on a runner over identical data.
    let mut names = toolchains()
        .iter()
        .map(|entry| entry.display_name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    names.sort();
    for pair in names.windows(2) {
        assert_ne!(
            pair[0], pair[1],
            "two toolchains both call themselves {}",
            pair[0]
        );
    }
}

#[test]
fn the_languages_a_toolchain_names_are_real() {
    // A typo in data/toolchains would otherwise fail the build, but this names
    // the invariant rather than leaving it to a compile error.
    for entry in toolchains() {
        assert!(
            !entry.languages.is_empty(),
            "{} serves no language",
            entry.id
        );
        for language in entry.languages {
            assert!(
                language_profile(language.id).is_some(),
                "{} names unknown language {}",
                entry.id,
                language.id
            );
        }
    }
}
