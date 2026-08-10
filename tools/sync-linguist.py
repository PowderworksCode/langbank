#!/usr/bin/env python3
"""Keep data/languages/ in step with GitHub linguist.

Langbank owns its language data; linguist is one of the sources it is checked
against. This tool does two jobs and refuses to confuse them:

  check   report every language and every detection token linguist knows and
          langbank does not. Exits non-zero if anything is missing, which is
          what CI runs.
  create  write a file for each language that has none yet. It never edits a
          file that already exists -- hand-written entries carry conventions,
          facets and comment syntax that no importer should touch, so a missing
          token in an existing file is reported for a person to add.

    tools/sync-linguist.py check  [--revision REV]
    tools/sync-linguist.py create [--revision REV]
"""
import argparse
import glob
import hashlib
import json
import os
import re
import sys
import urllib.request

import yaml

UPSTREAM = "github-linguist/linguist"
URL = "https://raw.githubusercontent.com/{repo}/{rev}/lib/linguist/languages.yml"
PIN = "data/sources/linguist.toml"

# linguist's `type` is coarser than LanguageRole and never names build files.
ROLE = {"programming": "programming", "markup": "markup", "data": "data", "prose": "documentation"}


def slug(name):
    """`C#` -> `c-sharp`, matching the ids the hand-written profiles use."""
    out = name.lower().replace("#", "-sharp").replace("++", "pp").replace("*", "-star")
    out = "".join(c if c.isalnum() else "-" for c in out)
    while "--" in out:
        out = out.replace("--", "-")
    return out.strip("-")


def pinned_revision():
    if not os.path.exists(PIN):
        return "main"
    return re.search(r'^revision = "([^"]+)"', open(PIN).read(), re.M).group(1)


def fetch(revision):
    url = URL.format(repo=UPSTREAM, rev=revision)
    raw = urllib.request.urlopen(url, timeout=60).read()
    return raw, hashlib.sha256(raw).hexdigest(), url


def upstream_languages(raw):
    """id -> {display, role, extensions, filenames, shebangs}, all of them."""
    out = {}
    for name, entry in yaml.safe_load(raw).items():
        role = ROLE.get(entry.get("type"))
        if role is None:
            continue
        out[slug(name)] = {
            "display": name,
            "role": role,
            "extensions": sorted({e.lstrip(".").lower() for e in entry.get("extensions", [])}),
            "filenames": sorted(set(entry.get("filenames", []))),
            "shebangs": sorted(set(entry.get("interpreters", []))),
        }
    return out


def local_languages():
    """id -> {path, extensions, filenames, shebangs} for what langbank carries."""
    out = {}
    for path in sorted(glob.glob("data/languages/*.toml")):
        text = open(path).read()
        lid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)

        def field(name):
            match = re.search(rf'^{name} = \[(.*?)\]', text, re.M | re.S)
            return set(re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1))) if match else set()

        out[lid] = {
            "path": path,
            "extensions": field("extensions"),
            "filenames": field("filenames"),
            "shebangs": field("shebangs"),
        }
    return out


def gaps(upstream, local):
    missing_languages = sorted(set(upstream) - set(local))
    missing_tokens = {}
    for lid, entry in upstream.items():
        if lid not in local:
            continue
        holes = {
            kind: sorted(set(entry[kind]) - local[lid][kind])
            for kind in ("extensions", "filenames", "shebangs")
            if set(entry[kind]) - local[lid][kind]
        }
        if holes:
            missing_tokens[lid] = holes
    return missing_languages, missing_tokens


def write_language(lid, entry, sources):
    lines = [f"id = {json.dumps(lid)}", f"display-name = {json.dumps(entry['display'])}",
             f"role = {json.dumps(entry['role'])}"]
    for key, values in (("extensions", entry["extensions"]),
                        ("filenames", entry["filenames"]),
                        ("shebangs", entry["shebangs"])):
        if values:
            lines.append(f"{key} = {json.dumps(values)}")
    lines.append(f"sources = {json.dumps(sources)}")
    open(f"data/languages/{lid}.toml", "w").write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    parser.add_argument("--revision", default=None)
    args = parser.parse_args()

    revision = args.revision or pinned_revision()
    raw, digest, url = fetch(revision)
    upstream = upstream_languages(raw)
    local = local_languages()
    missing_languages, missing_tokens = gaps(upstream, local)

    if args.command == "create":
        for lid in missing_languages:
            write_language(lid, upstream[lid], ["linguist"])
        os.makedirs("data/sources", exist_ok=True)
        open(PIN, "w").write(
            "# Upstream sources langbank is checked against. `tools/sync-linguist.py check`\n"
            "# fails if any of them knows a language or a detection token we do not.\n"
            f'\n[linguist]\nrepository = "{UPSTREAM}"\nrevision = "{revision}"\n'
            f'source = "{url}"\nsha256 = "{digest}"\nlicense = "MIT"\n'
            f'languages = {len(upstream)}\n'
        )
        print(f"created {len(missing_languages)} language files from linguist@{revision[:12]}")
        if missing_tokens:
            print(f"{len(missing_tokens)} existing files are missing tokens; run `check` to see them")
        return 0

    print(f"linguist@{revision[:12]}: {len(upstream)} languages, langbank has {len(local)}")
    if not missing_languages and not missing_tokens:
        print("coverage complete: langbank knows every language and token linguist does")
        return 0
    if missing_languages:
        print(f"\n{len(missing_languages)} languages missing:")
        for lid in missing_languages[:40]:
            print(f"  {lid}")
        if len(missing_languages) > 40:
            print(f"  … and {len(missing_languages) - 40} more")
    if missing_tokens:
        print(f"\n{len(missing_tokens)} languages missing detection tokens:")
        for lid, holes in sorted(missing_tokens.items()):
            for kind, values in holes.items():
                print(f"  {lid}: {kind} {' '.join(values)}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
