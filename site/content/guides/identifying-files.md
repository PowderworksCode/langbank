---
title: Identify a file
description: How langbank decides what a file is, and where the evidence comes from.
order: 1
---

Identification runs from the cheapest evidence to the dearest, and every
answer names the evidence that produced it.

## Names first

`detect_language` reads the path and, when you have one, the first bytes:

```rust
use langbank::detect_language;
use std::path::Path;

detect_language(Path::new("Dockerfile"), None);           // dockerfile, by filename
detect_language(Path::new("src/main.rs"), None);          // rust, by extension
detect_language(Path::new("deploy"), Some(b"#!/bin/sh")); // shell, by shebang
```

A filename that identifies a language outright wins. An extension answers next
— but only when it is uncontested, or when exactly one claimant declares it a
`primary` extension. A `#!` line naming an interpreter answers for files with
no telling name at all.

## Contested extensions decline to guess

A `.h` is C or C++; a `.pl` is Perl or Prolog. For those, the leaf crate
*states* ordered content rules — the same shape as linguist's heuristics — and
returns nothing rather than guessing. Declining is the feature: a wrong answer
propagates into every consumer that trusted it.

## Running the rules

Reading a file takes a regex engine, which the leaf must not carry. That is
`langbank-detect`, the crate that runs what the leaf describes:

```toml
[dependencies]
langbank-detect = { git = "https://github.com/PowderworksCode/langbank" }
```

It reports which rule fired, so a consumer can log *why* a `.h` was called
C++ rather than only that it was. How a language was decided matters as much
as the answer: a name is cheap and sometimes wrong, and reading the file is
neither.

## What the rules cover

The rules and the claims they arbitrate are data like everything else — the
[languages reference](/reference/languages/) marks which languages carry
detection, and disputes langbank cannot settle are on the
[gaps page](/reference/gaps/) with the reason each one stands.
