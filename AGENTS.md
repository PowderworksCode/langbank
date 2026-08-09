# Working in semiotics

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
- `cargo fmt`, `cargo clippy --all-targets` and `cargo test` are all expected to
  be clean before a commit.
