#!/usr/bin/env python3
"""Add package ecosystems from dependabot-core to data/ecosystems/.

Dependabot states its facts in Ruby, so `tools/extract-dependabot.rb` reads them
with Ripper -- Ruby's own parser -- and emits JSON. A regex would have to guess
at string boundaries and, worse, would miss the ecosystems that name a constant
instead of spelling a filename out: composer says PackageManager::
MANIFEST_FILENAME and deno says MANIFEST_FILENAMES. Following those references
took the yield from 16 ecosystems to 27.

What dependabot does not say is which language an ecosystem publishes for, or
which purl registry its packages live in. Those are the table below, written by
hand because they are judgements rather than extractions, and each one is a
claim somebody should be able to check.

Ecosystems dependabot updates that are not language package managers --
github_actions, git_submodules, devcontainers, pre_commit, terraform, docker --
are deliberately absent. Langbank's ecosystem is a thing that manages a
language's packages.

    tools/sync-dependabot.py check <facts.json>
    tools/sync-dependabot.py create <facts.json>
"""
import argparse
import glob
import json
import re
import sys

# slug -> (id, display name, languages, purl registry, roles)
KNOWN = {
    "bundler":   ("bundler", "Bundler", ["ruby"], "gem", ["package-manager"]),
    "composer":  ("composer", "Composer", ["php"], "composer", ["package-manager"]),
    "go_modules": ("go-modules", "Go modules", ["go"], "golang", ["package-manager", "build-system"]),
    "hex":       ("hex", "Hex", ["elixir"], "hex", ["package-manager"]),
    "maven":     ("maven", "Maven", ["java"], "maven", ["package-manager", "build-system"]),
    "gradle":    ("gradle", "Gradle", ["java", "kotlin"], "maven", ["package-manager", "build-system"]),
    "sbt":       ("sbt", "sbt", ["scala"], "maven", ["package-manager", "build-system"]),
    "pub":       ("pub", "Pub", ["dart"], "pub", ["package-manager"]),
    "swift":     ("swift-pm", "Swift Package Manager", ["swift"], "swift", ["package-manager", "build-system"]),
    "conda":     ("conda", "conda", ["python"], "conda", ["package-manager"]),
    "elm":       ("elm", "Elm packages", ["elm"], None, ["package-manager"]),
    "deno":      ("deno", "Deno", ["typescript", "javascript"], None, ["package-manager", "runtime"]),
    "bazel":     ("bazel", "Bazel", [], "bazel", ["build-system"]),
}


def plain(name):
    """A literal filename, not a regex dependabot matches with."""
    return not re.search(r"[\\^$*+?\[\]()|]", name) and "/" not in name


def existing():
    out = {}
    for path in sorted(glob.glob("data/ecosystems/*.toml")):
        text = open(path).read()
        out[re.search(r'^id = "([^"]+)"', text, re.M).group(1)] = path
    return out


def languages():
    return {
        re.search(r'^id = "([^"]+)"', open(path).read(), re.M).group(1)
        for path in sorted(glob.glob("data/languages/*.toml"))
    }


def registries():
    return {
        re.search(r'^id = "([^"]+)"', open(path).read(), re.M).group(1)
        for path in sorted(glob.glob("data/registries/*.toml"))
    }


def render(entry, facts):
    eco_id, display, langs, registry, roles = entry
    manifests = [f for f in facts.get("required_files", []) if plain(f)]
    locks = [f for f in facts.get("lockfiles", []) if plain(f)]
    lines = [f'id = "{eco_id}"']
    if registry:
        lines.append(f'registry = "{registry}"')
    lines += [
        f"display-name = {json.dumps(display, ensure_ascii=False)}",
        f"roles = {json.dumps(roles, ensure_ascii=False)}",
    ]
    if langs:
        lines.append(f"implied-languages = {json.dumps(langs, ensure_ascii=False)}")
    if manifests:
        # Langbank models one manifest per ecosystem; the rest of what
        # dependabot accepts is recorded as a selector so nothing is lost.
        lines.append(f"manifest = {json.dumps(manifests[0], ensure_ascii=False)}")
        if len(manifests) > 1:
            lines.append(f"selector-files = {json.dumps(manifests[1:], ensure_ascii=False)}")
    if locks:
        lines.append(f"lockfiles = {json.dumps(locks, ensure_ascii=False)}")
    lines.append('manifest-selection = "default"')
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    parser.add_argument("facts", help="output of tools/extract-dependabot.rb")
    args = parser.parse_args()

    facts = {entry["slug"]: entry for entry in json.load(open(args.facts))}
    have, known_languages, known_registries = existing(), languages(), registries()

    missing, bad = [], []
    for slug, entry in sorted(KNOWN.items()):
        eco_id, _, langs, registry, _ = entry
        if slug not in facts:
            bad.append(f"{slug}: dependabot no longer defines this ecosystem")
            continue
        for language in langs:
            if language not in known_languages:
                bad.append(f"{eco_id}: unknown language {language!r}")
        if registry and registry not in known_registries:
            bad.append(f"{eco_id}: {registry!r} is not a purl type")
        if eco_id not in have:
            missing.append((slug, entry))

    if args.command == "check":
        print(f"{len(KNOWN)} package ecosystems mapped from dependabot; "
              f"{len(have)} carried, {len(missing)} missing")
        for problem in bad:
            print(f"  {problem}")
        return 1 if missing or bad else 0

    if bad:
        for problem in bad:
            print(f"  {problem}")
        return 1
    for slug, entry in missing:
        open(f"data/ecosystems/{entry[0]}.toml", "w").write(render(entry, facts[slug]))
    print(f"created {len(missing)} ecosystems")
    return 0


if __name__ == "__main__":
    sys.exit(main())
