---
title: Crates
description: The workspace members, what each one is for, and what each may depend on.
order: 1
---

`langbank` is the leaf and stays one: a consumer takes it without taking a
network stack, a regex engine or an archive reader. Everything that needs
those is a separate member, and CI fails if the leaf gains a dependency.

| crate | what it is |
|---|---|
| `langbank` | the data, as `&'static` tables compiled from `data/**/*.toml` |
| `langbank-detect` | runs the content rules the leaf only describes, and reports which one fired |
| `langbank-sync` | `check` and `create` against each upstream; what keeps the data honest |
| `langbank-web` | the server that browses the registries and runs the rules against a pasted file |

None are on crates.io yet; take them by git:

```toml
[dependencies]
langbank = { git = "https://github.com/PowderworksCode/langbank" }
```

The leaf also exports everything it knows as one JSON document — the
[manifest](/langbank.json) this site is rendered from:

```sh
cargo run -p langbank --example manifest > langbank.json
```

An example target rather than a binary, so the exporter and its serde_json
stay dev-side: a consumer of the library takes neither.
