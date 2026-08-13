#!/usr/bin/env python3
"""Absorb mason-registry into data/toolchains/.

Mason is the inverse index of nvim-lspconfig. lspconfig knows how to *run* a
tool; mason knows what it *is* and how it is *distributed* -- and it says so in
purl, the same vocabulary data/registries/ already carries.

Two jobs, kept apart the way the other sync tools do:

  a package whose program langbank already knows gains its distribution and
  categories, appended, with nothing existing rewritten;

  a package langbank does not know becomes a new toolchain entry.

Mason names languages in prose -- `LaTeX`, `Bash`, `Terraform` -- so those are
mapped, and packages whose languages map to nothing are skipped rather than
filed against invented ids.

    tools/sync-mason.py check
    tools/sync-mason.py create
"""
import argparse
import glob
import json
import re
import sys
import tarfile
import urllib.parse
import urllib.request
from io import BytesIO

UPSTREAM = "mason-org/mason-registry"
TARBALL = f"https://codeload.github.com/{UPSTREAM}/tar.gz/refs/heads/main"

CATEGORY = {
    "LSP": "language-server", "Formatter": "formatter", "Linter": "linter",
    "DAP": "debugger", "Runtime": "runtime", "Compiler": "compiler",
}
# Mason writes language names for people to read.
ALIAS = {
    "bash": "shell", "sh": "shell", "latex": "tex", "terraform": "hcl",
    "docker": "dockerfile", "c#": "c-sharp", "c++": "cpp", "f#": "f-sharp",
    "objective-c": "objective-c", "golang": "go", "protobuf": "protocol-buffer",
    "javascript react": "javascript", "typescript react": "typescript",
}


def block(text, key):
    match = re.search(rf"^{key}:\n((?:  - .*\n)+)", text, re.M)
    return re.findall(r"^  - (.+)$", match.group(1), re.M) if match else []


def upstream_packages():
    raw = urllib.request.urlopen(TARBALL, timeout=240).read()
    out = []
    with tarfile.open(fileobj=BytesIO(raw)) as archive:
        for member in archive.getmembers():
            if not member.name.endswith("/package.yaml"):
                continue
            text = archive.extractfile(member).read().decode("utf-8", "replace")
            name = re.search(r"^name:\s*(\S+)", text, re.M)
            if not name:
                continue
            source = re.search(r"^\s*id:\s*(\S+)", text, re.M)
            out.append({
                "name": name.group(1),
                "languages": block(text, "languages"),
                "categories": block(text, "categories"),
                "purl": source.group(1) if source else None,
            })
    return sorted(out, key=lambda package: package["name"])


def langbank():
    languages, by_display = {}, {}
    for path in sorted(glob.glob("data/languages/*.toml")):
        text = open(path).read()
        lid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        display = re.search(r'^display-name = "([^"]+)"', text, re.M)
        languages[lid] = path
        by_display[(display.group(1) if display else lid).lower()] = lid
    toolchains, programs = {}, {}
    for path in sorted(glob.glob("data/toolchains/*.toml")):
        text = open(path).read()
        tid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        toolchains[tid] = (path, text)
        found = re.search(r"^programs = \[(.*?)\]", text, re.M)
        for program in re.findall(r'"([^"]+)"', found.group(1) if found else ""):
            programs.setdefault(program, tid)
        # lspconfig names a server by its own id; mason names the same tool by
        # its package. Matching on both is what stops one tool becoming two.
        display = re.search(r'^display-name = "([^"]+)"', text, re.M)
        if display:
            programs.setdefault(display.group(1), tid)
    return languages, by_display, toolchains, programs


def to_language(name, by_display):
    key = name.lower()
    return by_display.get(ALIAS.get(key, key)) or by_display.get(key)


def purl_parts(purl):
    """Split `pkg:type/namespace/name@version` into its type and its name.

    The version is after the *last* `@`, not the first. An npm scope is part of
    the name and is spelled either `@scope/name` or percent-encoded as
    `%40scope/name`, so a pattern that stops at the first `@` drops every scoped
    package on the floor — which is exactly what the first version did, losing
    the distribution for thirteen tools without saying anything.
    """
    match = re.match(r"pkg:([a-zA-Z0-9.+-]+)/(.+)", purl or "")
    if not match:
        return (None, None)
    kind, rest = match.group(1), match.group(2)
    rest = rest.split("?")[0].split("#")[0]
    at = rest.rfind("@")
    if at > 0:
        rest = rest[:at]
    return (kind, urllib.parse.unquote(rest))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    args = parser.parse_args()

    packages = upstream_packages()
    _, by_display, toolchains, programs = langbank()
    registries = {
        re.search(r'^id = "([^"]+)"', open(path).read(), re.M).group(1)
        for path in sorted(glob.glob("data/registries/*.toml"))
    }

    merges, creates, skipped = [], [], 0
    for package in packages:
        languages = sorted({l for name in package["languages"] if (l := to_language(name, by_display))})
        if not languages:
            skipped += 1
            continue
        kinds = [CATEGORY[c] for c in package["categories"] if c in CATEGORY]
        registry, name = purl_parts(package["purl"])
        entry = {**package, "mapped": languages, "kinds": kinds,
                 "registry": registry, "package": name}
        existing = programs.get(package["name"])
        if existing:
            _, text = toolchains[existing]
            # Outstanding only when there is something to write. A package with
            # no purl gets no distribution block, so counting it as pending
            # would leave the check permanently red — and a check that cannot
            # go green teaches everyone to ignore it.
            wants_distribution = registry and name and "[distribution]" not in text
            wants_categories = kinds and "categories" not in text
            if wants_distribution or wants_categories:
                merges.append((existing, entry))
        elif f'mason-{package["name"]}' not in toolchains:
            creates.append(entry)

    if args.command == "check":
        print(f"{len(packages)} mason packages; {len(merges)} would add distribution to a "
              f"known tool, {len(creates)} are new, {skipped} skipped for unmapped languages")
        seen = [entry for _, entry in merges] + creates
        unknown = {entry["registry"] for entry in seen} - registries - {None}
        if unknown:
            print(f"  purl types mason uses that purl does not define: {sorted(unknown)}")
        return 1 if merges or creates else 0

    for tid, entry in merges:
        path, text = toolchains[tid]
        lines = []
        # Append only what is absent. `create` has to be safe to run twice —
        # the first version appended unconditionally and produced files with
        # `categories` written twice, which TOML rejects as overwriting a value.
        if entry["kinds"] and "categories" not in text:
            lines.append(f'categories = {json.dumps(entry["kinds"], ensure_ascii=False)}')
        # a package with no purl has no distribution to record, and a null is
        # not a fact
        if entry["registry"] and entry["package"] and "[distribution]" not in text:
            lines.append(f'\n[distribution]\nregistry = {json.dumps(entry["registry"], ensure_ascii=False)}')
            lines.append(f'package = {json.dumps(entry["package"], ensure_ascii=False)}')
        if lines:
            open(path, "a").write("\n".join(lines) + "\n")

    for entry in creates:
        kind = entry["kinds"][0] if entry["kinds"] else "linter"
        lines = [
            f'id = "mason-{entry["name"]}"',
            f'display-name = {json.dumps(entry["name"], ensure_ascii=False)}',
            f'kind = "{kind}"',
            f'languages = {json.dumps(entry["mapped"], ensure_ascii=False)}',
            f'programs = {json.dumps([entry["name"]], ensure_ascii=False)}',
        ]
        if entry["kinds"]:
            lines.append(f'categories = {json.dumps(entry["kinds"], ensure_ascii=False)}')
        if entry["registry"] and entry["package"]:
            lines.append(f'\n[distribution]\nregistry = {json.dumps(entry["registry"], ensure_ascii=False)}')
            lines.append(f'package = {json.dumps(entry["package"], ensure_ascii=False)}')
        open(f'data/toolchains/mason-{entry["name"]}.toml', "w").write("\n".join(lines) + "\n")

    print(f"merged distribution into {len(merges)} known tools, created {len(creates)} new, "
          f"{skipped} skipped for unmapped languages")
    return 0


if __name__ == "__main__":
    sys.exit(main())
