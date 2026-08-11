#!/usr/bin/env python3
"""Keep data/registries/ in step with package-url/purl-spec.

A purl type is a *package registry*: the namespace a package identity lives in,
`pkg:npm/lodash@4`. It is not a package manager -- npm, pnpm, yarn and bun are
four managers over one registry -- and keeping the two apart is the whole reason
this data is separate from data/ecosystems/.

    tools/sync-purl.py check    report types purl defines and langbank lacks
    tools/sync-purl.py create   write a file for each type we lack
"""
import argparse
import glob
import json
import os
import re
import sys
import urllib.request


def dumps(value):
    """TOML takes literal UTF-8; json.dumps escapes non-BMP into surrogate pairs."""
    return json.dumps(value, ensure_ascii=False)

INDEX = "https://raw.githubusercontent.com/package-url/purl-spec/main/purl-types-index.json"
TYPE = "https://raw.githubusercontent.com/package-url/purl-spec/main/types/{name}-definition.json"
PIN = "data/sources/purl.toml"


def fetch(url):
    return urllib.request.urlopen(url, timeout=60).read()


def upstream_types():
    names = json.loads(fetch(INDEX))
    return {name: json.loads(fetch(TYPE.format(name=name))) for name in names}


def local_types():
    out = {}
    for path in sorted(glob.glob("data/registries/*.toml")):
        text = open(path).read()
        out[re.search(r'^id = "([^"]+)"', text, re.M).group(1)] = path
    return out


def requirement(definition, key):
    part = definition.get(f"{key}_definition") or {}
    return part.get("requirement", "optional"), bool(part.get("case_sensitive", True))


def write_type(name, definition):
    repository = definition.get("repository") or {}
    lines = [
        f"id = {dumps(name)}",
        f"display-name = {dumps(definition.get('type_name') or name)}",
    ]
    url = repository.get("default_repository_url")
    if url:
        lines.append(f"default-repository = {dumps(url)}")
    lines.append(f"uses-repository = {dumps(bool(repository.get('use_repository', False)))}")
    for key in ("namespace", "name", "version"):
        required, case_sensitive = requirement(definition, key)
        lines.append(f"\n[{key}]")
        lines.append(f'requirement = "{required}"')
        lines.append(f"case-sensitive = {dumps(case_sensitive)}")
    open(f"data/registries/{name}.toml", "w").write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    args = parser.parse_args()

    upstream = upstream_types()
    local = local_types()
    missing = sorted(set(upstream) - set(local))
    extra = sorted(set(local) - set(upstream))

    if args.command == "create":
        for name in missing:
            write_type(name, upstream[name])
        os.makedirs("data/sources", exist_ok=True)
        open(PIN, "w").write(
            "# purl-spec defines the package registries a package identity can live in.\n"
            "# `tools/sync-purl.py check` fails if it defines a type langbank does not.\n"
            '\n[purl]\nrepository = "package-url/purl-spec"\n'
            f'source = "{INDEX}"\nlicense = "MIT"\ntypes = {len(upstream)}\n'
        )
        print(f"created {len(missing)} registry files from purl-spec")
        return 0

    print(f"purl-spec defines {len(upstream)} types, langbank carries {len(local)}")
    if extra:
        print(f"\n{len(extra)} registries langbank has that purl does not define:")
        for name in extra:
            print(f"  {name}")
    if missing:
        print(f"\n{len(missing)} purl types missing:")
        for name in missing:
            print(f"  {name}")
        return 1
    if not extra:
        print("coverage complete: langbank carries every purl type")
    return 0


if __name__ == "__main__":
    sys.exit(main())
