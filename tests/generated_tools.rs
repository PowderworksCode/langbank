//! Tool profiles, generated from `data/tools/`.
//!
//! The profiles are only interesting through `classify_tool`, which matches a
//! real invocation against the command patterns and reports what it does and
//! what it produces. So these test invocations rather than fields: the command
//! patterns carry required and rejected argument matching, and a generator
//! that dropped either would still produce a registry that looks right.

use langbank::*;

fn classify(program: &str, arguments: &[&str]) -> Vec<(TaskKind, Vec<&'static str>)> {
    let arguments = arguments
        .iter()
        .map(|a| (*a).to_owned())
        .collect::<Vec<_>>();
    classify_tool(program, &arguments)
        .into_iter()
        .map(|(_, task, artifacts)| (task, artifacts.iter().map(|artifact| artifact.id).collect()))
        .collect()
}

#[test]
fn every_tool_file_produced_a_profile() {
    assert_eq!(
        tool_profiles().len(),
        17,
        "one profile per file in data/tools"
    );
}

#[test]
fn a_command_maps_to_its_tasks_and_artifacts() {
    assert_eq!(
        classify("cargo", &["test"]),
        vec![(TaskKind::Test, vec!["binary"])]
    );
    assert_eq!(
        classify("cargo", &["clippy"]),
        vec![(TaskKind::Lint, vec![])]
    );
    assert_eq!(
        classify("cargo", &["fmt"]),
        vec![(TaskKind::Format, vec![])]
    );
    assert_eq!(
        classify("cargo", &["build"]),
        vec![(TaskKind::Build, vec!["binary"])]
    );
    // `check` builds nothing, which is the distinction worth keeping
    assert_eq!(
        classify("cargo", &["check"]),
        vec![(TaskKind::Build, vec![])]
    );
    assert!(classify("cargo", &["publish"]).is_empty());
    assert!(classify("not-a-tool", &["test"]).is_empty());
}

#[test]
fn required_arguments_decide_which_artifact_a_build_produces() {
    // `bun build` on its own produces nothing identifiable
    assert_eq!(classify("bun", &["build"]), vec![]);
    // with --compile it is a binary, exact or prefixed
    assert_eq!(
        classify("bun", &["build", "--compile"]),
        vec![(TaskKind::Build, vec!["binary"])]
    );
    assert_eq!(
        classify("bun", &["build", "--compile=./out"]),
        vec![(TaskKind::Build, vec!["binary"])]
    );
}

#[test]
fn rejected_arguments_stop_a_pattern_from_claiming_a_command() {
    // --target browser is a site...
    assert_eq!(
        classify("bun", &["build", "--target", "browser"]),
        vec![(TaskKind::Build, vec!["site"])]
    );
    assert_eq!(
        classify("bun", &["build", "--target=browser"]),
        vec![(TaskKind::Build, vec!["site"])]
    );
    // ...unless --compile is also present, in which case the site pattern is
    // rejected and only the binary one matches. Losing rejected_arguments in
    // the generator would report both, which is the failure this catches.
    assert_eq!(
        classify("bun", &["build", "--target", "browser", "--compile"]),
        vec![(TaskKind::Build, vec!["binary"])]
    );
}

#[test]
fn a_tool_covers_every_program_that_spells_it() {
    // one profile, four package managers
    for program in ["bun", "npm", "pnpm", "yarn"] {
        assert_eq!(
            classify(program, &["test"]),
            vec![(TaskKind::Test, vec![])],
            "{program} test"
        );
    }
}

#[test]
fn test_retry_configurations_keep_their_paths_and_signals() {
    let cargo = tool_profile("cargo").expect("cargo");
    let retry = cargo.test_retry.expect("cargo declares retry");
    assert_eq!(retry.arguments, &["--retries"]);
    assert_eq!(retry.configurations.len(), 1);
    assert_eq!(retry.configurations[0].paths, &[".config/nextest.toml"]);
    assert!(matches!(
        retry.configurations[0].signals,
        [TestRetrySignal::TomlPositiveInteger("retries")]
    ));

    // the JavaScript runner carries three configurations and two signal kinds
    let runner = tool_profile("javascript-test-runner").expect("runner");
    let retry = runner.test_retry.expect("runner declares retry");
    assert_eq!(retry.configurations.len(), 3);
    let jest = retry
        .configurations
        .iter()
        .find(|configuration| configuration.paths.contains(&"jest.config.ts"))
        .expect("jest configuration");
    assert!(matches!(
        jest.signals,
        [
            TestRetrySignal::JavascriptProperty("retries"),
            TestRetrySignal::JavascriptCall("retryTimes"),
        ]
    ));

    assert!(
        tool_profile("rustfmt")
            .expect("rustfmt")
            .test_retry
            .is_none()
    );
}

#[test]
fn tools_point_at_language_profiles_not_copies() {
    let cargo = tool_profile("cargo").expect("cargo");
    let rust = language_profile("rust").expect("rust");
    assert!(
        cargo
            .languages
            .iter()
            .any(|language| std::ptr::eq(*language, rust))
    );
    assert_eq!(cargo.ci_workload, CiWorkload::Heavy);
    assert_eq!(
        tool_profile("rustfmt").expect("rustfmt").ci_workload,
        CiWorkload::Light
    );
}

#[test]
fn the_named_exports_still_resolve() {
    // lib.rs re-exports these three by name; the generator renamed the modules
    // they used to live in, so this is the guard on that.
    assert_eq!(CODESPELL.id, "codespell");
    assert_eq!(VALE.id, "vale");
    assert_eq!(STYLELINT.id, "stylelint");
}
