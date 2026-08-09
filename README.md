# Semiotics

Language, ecosystem and toolchain data: what a language is, how to recognise
it, what conventions it carries, which ecosystem publishes it, and what its
tooling can be asked.

Everything here is a registry over static data plus the few functions needed to
look something up. Nothing here walks a filesystem, spawns a process, or parses
a source file.

## Why it is its own repository

It sits at the bottom of the fleet. Entl names languages while walking a tree,
treebank names them when it publishes a grammar, and propbank names them when
it observes a program. All three need the same vocabulary, and none of them
should have to depend on another to get it.

```
                    semiotics
                 ↑      ↑      ↑
              entl  treebank  propbank
```

Every arrow points in. Nothing here depends on anything else in the fleet.

## What is in it today

Lifted from `entl-codebase/src/profiles`, essentially unchanged:

| registry | count |
|---|---|
| languages | 29 |
| ecosystems | 5 — cargo, npm, pnpm, yarn, bun |
| tool profiles | codespell, vale, stylelint, and the rust/javascript/system/tauri sets |
| artifacts | binary, napi, site, tauri |
| facets | structured-code, style-host, component-host |
| conventions | test layout, inline-test detection, typecheck defaults |
| verbosity | measured relative verbosity per language and per language pair |
| traversal | registered pruning directories |

```rust
use semiotics::{detect_language, language_profile, verbosity_ratio};

detect_language(Path::new("src/main.rs"), None);          // -> rust, by extension
detect_language(Path::new("deploy"), Some(b"#!/bin/sh")); // -> shell, by shebang
language_profile("rust").and_then(|p| p.conventions);     // test layout, inline tests
verbosity_ratio("rust", "typescript");                    // measured, corpus-versioned
```

Registration goes through `inventory`, so a downstream crate can add profiles
without editing this one.

## Direction

Stated as intent rather than schedule.

1. **Move the data out of Rust and into TOML**, generating today's statics from
   it at build time. The data becomes reviewable, generable, validatable, and
   exportable to consumers that are not Rust — without paying a runtime cost.
2. **Bootstrap detection breadth from GitHub linguist**, whose `languages.yml`
   is MIT and already the de-facto standard, keeping the hand-modelled depth
   layer for the languages this fleet actually works on. Two layers, two
   evidence bars, and the difference recorded rather than blurred.
3. **Model toolchains**, which exist nowhere today: compiler identity and
   version probes, invocation patterns for build/test/typecheck/format/lint,
   registry and popularity sources, and machine-readable diagnostic formats.
   treebank has all of this hardcoded per language in Rust, and propbank needs
   the version probes for its staleness checks.
4. **Absorb treebank's registry data** — crates.io dumps, npm, Maven Central,
   NuGet, Debian popcon, `packages.ecosyste.ms` — which is `rank`/`resolve`
   today and is plainly data.

## Provenance

`profiles/` came from entl, whose design doc drew the line this repository
makes structural:

> "Language profiles and ecosystem profiles are separate registries. An
> ecosystem role is not a language, and a language is not an ecosystem."

One behavioural change during the lift: `DependencyPinPolicy::classify` took a
parsed `Dependency` and now takes a `DependencySource` and a requirement.
A pin policy needs the taxonomy and the spec; the parsed record belongs to
whoever read the manifest, and borrowing it would drag manifest parsing down
here. Covered by `tests/registries.rs`.
