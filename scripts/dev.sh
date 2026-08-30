#!/usr/bin/env bash
# Stand up a fresh langbank checkout: hooks, then the workspace and the gate CI
# runs over it. Safe to re-run; every step is idempotent.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# A clone runs no hooks until it is pointed at them: core.hooksPath is per-clone
# configuration, so nothing a checkout carries can set it for you.
git config core.hooksPath .githooks
if [ ! -d .githooks ]; then
    echo "note: .githooks is fleet-managed and not synced here yet; git will"
    echo "      start using it the moment ordnung writes it."
fi

if ! command -v cargo >/dev/null; then
    echo "error: cargo is not on PATH; install Rust from https://rustup.rs" >&2
    exit 1
fi

# build.rs compiles data/**/*.toml into the &'static tables, so this is also
# what tells you a data file is malformed. rust-toolchain.toml names the
# channel, and rustup installs it on first use.
echo "== workspace"
cargo build --workspace --all-targets

echo "== fmt"
cargo fmt --check

# --workspace throughout: without it, clippy and the tests see only the root
# package and the other three members go unexamined.
echo "== clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "== test"
cargo test --workspace

# The invariant the whole layout exists to protect, checked here for the same
# reason CI checks it: the easiest way to lose it is one convenient `cargo add`
# in the wrong directory, and that is a thing you do locally.
echo "== the leaf stays a leaf"
if command -v jq >/dev/null; then
    allowed='inventory serde serde_json toml'
    actual=$(cargo metadata --format-version 1 --no-deps |
        jq -r '.packages[] | select(.name == "langbank") | .dependencies[].name' | sort -u)
    for dep in $actual; do
        case " $allowed " in
        *" $dep "*) ;;
        *)
            echo "error: langbank gained a runtime dependency: $dep" >&2
            exit 1
            ;;
        esac
    done
    echo "langbank depends only on: $(echo "$actual" | tr '\n' ' ')"
else
    echo "skipped: jq is not installed"
fi

echo
echo "ready. the rest of what CI checks:"
echo "  straitjacket    # the taste rules, excused in straitjacket.toml"
echo
echo "and what keeps the data honest, a source at a time:"
echo "  cargo run -p langbank-sync -- coverage         # what langbank knows"
echo "  cargo run -p langbank-sync -- linguist check   # against one upstream"
