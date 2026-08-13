#!/usr/bin/env python3
"""Absorb analysis-tools-dev/static-analysis into data/toolchains/.

755 linters and formatters, curated per language. It is largely disjoint from
mason -- 666 of them are tools langbank does not otherwise know -- because mason
indexes what an editor can install and this indexes what an analyser community
has written.

Same split as the other sync tools: a tool whose program langbank already knows
gains its categories, appended, with nothing rewritten; a tool it does not know
becomes a new entry. Tools categorised only as `meta` or `performance` are
skipped, since those are collections and benchmarks rather than something a
language can be asked through.

    tools/sync-static-analysis.py check
    tools/sync-static-analysis.py create
"""
import argparse
import glob
import json
import re
import sys
import tarfile
import urllib.request
from io import BytesIO

UPSTREAM = "analysis-tools-dev/static-analysis"
TARBALL = f"https://codeload.github.com/{UPSTREAM}/tar.gz/refs/heads/master"
CATEGORY = {"linter": "linter", "formatter": "formatter"}

# The upstream tags are already close to langbank ids; these are the strays.
ALIAS = {
    "c++": "cpp", "c#": "c-sharp", "objective-c": "objective-c", "bash": "shell",
    "shell": "shell", "docker": "dockerfile", "terraform": "hcl", "latex": "tex",
    "golang": "go", "node": "javascript", "vue": "vue", "dotnet": "c-sharp",
    "protobuf": "protocol-buffer", "config": None, "ci": None, "security": None,
    "all": None, "multi": None,
}


def slug(name):
    out = "".join(c if c.isalnum() else "-" for c in name.lower())
    while "--" in out:
        out = out.replace("--", "-")
    return out.strip("-")


def upstream_tools():
    raw = urllib.request.urlopen(TARBALL, timeout=240).read()
    out = []
    with tarfile.open(fileobj=BytesIO(raw)) as archive:
        for member in archive.getmembers():
            if not re.search(r"/data/tools/[^/]+\.yml$", member.name):
                continue
            text = archive.extractfile(member).read().decode("utf-8", "replace")

            def block(key):
                match = re.search(rf"^{key}:\n((?:  - .*\n)+)", text, re.M)
                return (
                    [x.strip().strip("'\"") for x in re.findall(r"^  - (.+)$", match.group(1), re.M)]
                    if match
                    else []
                )

            name = re.search(r"^name:\s*(.+)$", text, re.M)
            if not name:
                continue
            out.append({
                "name": name.group(1).strip().strip("'\""),
                "categories": block("categories"),
                "tags": block("tags"),
            })
    return sorted(out, key=lambda tool: tool["name"].lower())


def langbank():
    by_display = {}
    for path in glob.glob("data/languages/*.toml"):
        text = open(path).read()
        lid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        by_display[lid] = lid
        display = re.search(r'^display-name = "([^"]+)"', text, re.M)
        if display:
            by_display[display.group(1).lower()] = lid
    toolchains, programs = {}, {}
    for path in glob.glob("data/toolchains/*.toml"):
        text = open(path).read()
        tid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        toolchains[tid] = (path, text)
        found = re.search(r"^programs = \[(.*?)\]", text, re.M)
        for program in re.findall(r'"([^"]+)"', found.group(1) if found else ""):
            programs.setdefault(program.lower(), tid)
        display = re.search(r'^display-name = "([^"]+)"', text, re.M)
        if display:
            programs.setdefault(display.group(1).lower(), tid)
    return by_display, toolchains, programs


def plan(tools, by_display, toolchains, programs):
    merges, creates, skipped = [], [], 0
    for tool in tools:
        kinds = [CATEGORY[c] for c in tool["categories"] if c in CATEGORY]
        if not kinds:
            skipped += 1
            continue
        languages = sorted({
            l for tag in tool["tags"]
            if (mapped := ALIAS.get(tag.lower(), tag.lower())) is not None
            and (l := by_display.get(mapped))
        })
        if not languages:
            skipped += 1
            continue
        entry = {**tool, "kinds": kinds, "languages": languages, "id": f"sa-{slug(tool['name'])}"}
        existing = programs.get(tool["name"].lower())
        if existing:
            _, text = toolchains[existing]
            if "categories" not in text:
                merges.append((existing, entry))
        elif entry["id"] not in toolchains:
            creates.append(entry)
    return merges, creates, skipped


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    args = parser.parse_args()

    tools = upstream_tools()
    by_display, toolchains, programs = langbank()
    merges, creates, skipped = plan(tools, by_display, toolchains, programs)

    if args.command == "check":
        print(f"{len(tools)} tools upstream; {len(merges)} would gain categories, "
              f"{len(creates)} are new, {skipped} skipped (no analysable category or language)")
        return 1 if merges or creates else 0

    for tid, entry in merges:
        path, text = toolchains[tid]
        if "categories" not in text:
            open(path, "a").write(
                f'categories = {json.dumps(entry["kinds"], ensure_ascii=False)}\n'
            )
    for entry in creates:
        lines = [
            f'id = "{entry["id"]}"',
            f'display-name = {json.dumps(entry["name"], ensure_ascii=False)}',
            f'kind = "{entry["kinds"][0]}"',
            f'languages = {json.dumps(entry["languages"], ensure_ascii=False)}',
            f'programs = {json.dumps([entry["name"].lower()], ensure_ascii=False)}',
            f'categories = {json.dumps(entry["kinds"], ensure_ascii=False)}',
        ]
        open(f'data/toolchains/{entry["id"]}.toml', "w").write("\n".join(lines) + "\n")
    print(f"gave categories to {len(merges)} known tools, created {len(creates)} new, "
          f"{skipped} skipped")
    return 0


if __name__ == "__main__":
    sys.exit(main())
