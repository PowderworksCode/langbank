---
title: Getting started
description: Add the crate, look a language up, and identify a file — end to end.
order: 1
---

This tutorial takes you from nothing to your first lookup. By the end you'll
have langbank in a project, know how to ask it what a file is, and know where
the machinery lives when a name alone cannot answer.

## 1. Add the crate

Langbank is not on crates.io yet; take it from the repository:

```toml
[dependencies]
langbank = { git = "https://github.com/PowderworksCode/langbank" }
```

The leaf crate is deliberately small: no filesystem walking, no process
spawning, no source parsing, and no dependency on anything else in the fleet.
CI fails if it gains one.

## 2. Look a language up

Everything hangs off a language id — lowercase, as linguist spells it:

```rust
use langbank::language_profile;

let rust = language_profile("rust").unwrap();
assert!(rust.extensions.contains(&"rs"));
```

A profile carries what langbank knows: extensions and filenames, the role the
language plays, comment syntax, conventions, and which languages it
supersedes. Most entries carry less than that — the
[languages reference](/reference/languages/) says exactly which of the
[eight facets](/reference/facets/) each one knows, and an absence is
[recorded with its reason](/reference/gaps/) rather than papered over.

## 3. Identify a file

Detection reads the name first — filename, then extension, then shebang:

```rust
use langbank::detect_language;
use std::path::Path;

detect_language(Path::new("src/main.rs"), None);          // rust, by extension
detect_language(Path::new("deploy"), Some(b"#!/bin/sh")); // shell, by shebang
```

Some extensions are contested — a `.h` is C or C++, a `.pl` is Perl or Prolog
— and for those the leaf only *states* the content rules. Running them takes a
regex engine, which is why that lives in a separate crate:

```toml
[dependencies]
langbank-detect = { git = "https://github.com/PowderworksCode/langbank" }
```

`langbank-detect` runs the rules the leaf describes and reports which one
fired, so an answer always names its evidence. The
[identifying files guide](/guides/identifying-files/) walks the whole
decision.

## Where to go next

- [Looking things up](/guides/looking-things-up/) — the registries beyond
  languages: ecosystems, toolchains, package registries, tools.
- [Reference](/reference/) — every registry, rendered from the data itself.
- [How the data stays honest](/guides/keeping-data-honest/) — what `sync`
  checks against upstream, and why gaps are recorded instead of filled.
