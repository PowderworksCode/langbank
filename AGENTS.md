# Working in langbank

**This crate depends on nothing else in the fleet, and that is the invariant
worth protecting.** entl, treebank and propbank all depend on it. If a change
here needs a type from one of them, the type is in the wrong place.

- No filesystem walking, no process spawning, no source parsing. Data and
  lookups only. Anything that needs to *run* something belongs to a consumer.
- Registries stay separate. An ecosystem role is not a language, a language is
  not a toolchain, an artifact is not either.
- Registration goes through `inventory`, which collects at link time, so a
  library that compiles proves nothing about whether its profiles registered.
  Facts are tested from `tests/`, which links the whole crate.
- **`data/` is the source of truth** for languages, facets, artifacts,
  ecosystems, traversal and tool profiles. `build.rs` generates the statics;
  never edit the generated output, and never add an entry by writing Rust.
- A directory an ecosystem generates is declared in that ecosystem's file, so
  the two cannot drift apart.
- A build script that cannot read its data panics on purpose. An empty registry
  that compiles is worse than a build failure, because a consumer sees a
  language-free world and no error.
- `cargo fmt`, `cargo clippy --all-targets` and `cargo test` are all expected to
  be clean before a commit.
