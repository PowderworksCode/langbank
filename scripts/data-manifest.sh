#!/usr/bin/env bash
# Regenerate site/content/langbank.json from the compiled tables.
#
# The manifest is what the documentation site renders, so it has to come out
# of the crate rather than be maintained beside it. The exporter is an example
# target — the leaf gains no dependency and no binary; a consumer takes the
# tables alone.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT=site/content/langbank.json
CHECK=0
[[ ${1:-} == "--check" ]] && CHECK=1

cargo run --quiet -p langbank --example manifest >"${OUT}.new"

if [[ $CHECK -eq 1 ]]; then
    if ! diff -u "$OUT" "${OUT}.new"; then
        rm -f "${OUT}.new"
        echo "data-manifest: $OUT is stale. Run scripts/data-manifest.sh and commit the result." >&2
        exit 1
    fi
    rm -f "${OUT}.new"
    echo "data-manifest: $OUT matches the crate"
else
    mv "${OUT}.new" "$OUT"
    echo "data-manifest: wrote $OUT"
fi
