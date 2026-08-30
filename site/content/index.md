[Get started](/getting-started/) · [Read the guides](/guides/) · [Browse the data](/reference/languages/) · [GitHub](https://github.com/PowderworksCode/langbank)

Langbank is language, ecosystem and toolchain data: what a language is, how to
recognise it, what conventions it carries, which ecosystem publishes it, and
what its tooling answers. Everything is a registry over static data plus
the few functions needed to look something up — nothing here walks a
filesystem, spawns a process, or parses a source file.

```rust
use langbank::{detect_language, language_profile};

detect_language(Path::new("src/main.rs"), None);          // -> rust, by extension
detect_language(Path::new("deploy"), Some(b"#!/bin/sh")); // -> shell, by shebang
language_profile("rust").and_then(|p| p.conventions);     // test layout, inline tests
```

The registries are hand-curated and relational: ecosystems point at languages,
tools point at languages and artifacts, languages point at facets and at the
languages they supersede. Nothing is derived from a corpus, and there is no
pipeline that regenerates it — this is the stable leaf the rest of
[the fleet](https://powderworks.dev) names things in.

Every page under [reference](/reference/) that lists the data is rendered from
[one JSON manifest](/langbank.json) the crate itself exports, so the website
can disagree with the code only by failing to build. Fetch that manifest if
you want the data without the crate.
