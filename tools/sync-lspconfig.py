#!/usr/bin/env python3
"""Absorb language servers from nvim-lspconfig into data/toolchains/.

A language server is a toolchain entry: a program, the languages it serves, and
the files it looks for to decide where a project begins. Root markers are
recorded on the server rather than on the language, because they are the
server's convention -- clangd wants compile_commands.json, ts_ls wants a
lockfile -- and aggregating them per language produces noise rather than fact.

Two exclusions, both deliberate:

  servers with ten or more filetypes are generic tooling -- formatters,
  spellcheckers, ast-grep -- whose root markers are their own config files and
  say nothing about any language;

  servers with no `cmd` need a locally installed path and cannot be described
  as a program to look for.

Servers that compute their root imperatively (`root_dir = function`) are still
carried, for their identity and command, with no markers. rust_analyzer, gopls
and jdtls are all in that group.

    tools/sync-lspconfig.py check
    tools/sync-lspconfig.py create
"""
import argparse
import glob
import json
import re
import sys
import tarfile
import urllib.request
from io import BytesIO

UPSTREAM = "neovim/nvim-lspconfig"
TARBALL = f"https://codeload.github.com/{UPSTREAM}/tar.gz/refs/heads/master"
GENERIC_FILETYPES = 10

# Neovim filetypes are close to langbank ids and not identical.
ALIAS = {
    "cs": "c-sharp", "sh": "shell", "bash": "shell", "zsh": "shell",
    "javascriptreact": "javascript", "typescriptreact": "typescript",
    "objc": "objective-c", "objcpp": "objective-cpp",
    "plaintex": "tex", "gomod": "go", "gowork": "go", "gotmpl": "go",
    "eruby": "html-erb", "make": "makefile", "yml": "yaml",
    "jsonc": "json-with-comments", "vb": "visual-basic-net", "ps1": "powershell",
    "rmd": "rmarkdown", "terraform": "hcl", "tf": "hcl",
}


def lua_list(body, key):
    """A Lua list literal, by brace matching. Regex alone reads past the
    closing brace and swallows the next field, which silently turns filetypes
    into root markers."""
    match = re.search(rf"\b{key}\s*=\s*\{{", body)
    if not match:
        return None
    start, depth = match.end() - 1, 0
    for index in range(start, len(body)):
        if body[index] == "{":
            depth += 1
        elif body[index] == "}":
            depth -= 1
            if depth == 0:
                return re.findall(r"'((?:[^'\\]|\\.)*)'", body[start:index])
    return None


def upstream_servers():
    raw = urllib.request.urlopen(TARBALL, timeout=180).read()
    out = []
    with tarfile.open(fileobj=BytesIO(raw)) as archive:
        for member in archive.getmembers():
            if not re.search(r"/lsp/[^/]+\.lua$", member.name):
                continue
            text = archive.extractfile(member).read().decode("utf-8", "replace")
            body = text[text.find("\nreturn {"):] if "\nreturn {" in text else text
            out.append({
                "id": member.name.rsplit("/", 1)[1][:-4],
                "cmd": lua_list(body, "cmd") or [],
                "filetypes": lua_list(body, "filetypes") or [],
                "markers": lua_list(body, "root_markers"),
            })
    return sorted(out, key=lambda server: server["id"])


def languages():
    return {
        re.search(r'^id = "([^"]+)"', open(path).read(), re.M).group(1)
        for path in sorted(glob.glob("data/languages/*.toml"))
    }


def usable(servers, known):
    def to_language(filetype):
        base = filetype.split(".")[0]
        candidate = ALIAS.get(base, base)
        return candidate if candidate in known else None

    out, dropped = [], {"generic": 0, "unmapped": 0, "no-command": 0}
    for server in servers:
        if len(server["filetypes"]) >= GENERIC_FILETYPES:
            dropped["generic"] += 1
            continue
        mapped = sorted({l for ft in server["filetypes"] if (l := to_language(ft))})
        if not mapped:
            dropped["unmapped"] += 1
            continue
        if not server["cmd"]:
            dropped["no-command"] += 1
            continue
        out.append({**server, "languages": mapped})
    return out, dropped


def write(server):
    lines = [
        f'id = "lsp-{server["id"].replace("_", "-")}"',
        f'display-name = {json.dumps(server["id"], ensure_ascii=False)}',
        'kind = "language-server"',
        f'languages = {json.dumps(server["languages"], ensure_ascii=False)}',
        f'programs = {json.dumps(server["cmd"][:1], ensure_ascii=False)}',
    ]
    if server["markers"]:
        lines.append(f'root-markers = {json.dumps(server["markers"], ensure_ascii=False)}')
    path = f'data/toolchains/lsp-{server["id"].replace("_", "-")}.toml'
    open(path, "w").write("\n".join(lines) + "\n")
    return path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    args = parser.parse_args()

    servers, known = upstream_servers(), languages()
    keep, dropped = usable(servers, known)
    have = {
        re.search(r'^id = "([^"]+)"', open(path).read(), re.M).group(1)
        for path in sorted(glob.glob("data/toolchains/*.toml"))
    }
    missing = [s for s in keep if f'lsp-{s["id"].replace("_", "-")}' not in have]

    if args.command == "create":
        for server in missing:
            write(server)
        print(f"wrote {len(missing)} language servers")
        return 0

    print(f"{len(servers)} servers upstream, {len(keep)} usable, langbank carries "
          f"{len(keep) - len(missing)}")
    print(f"  dropped: {dropped['generic']} generic, {dropped['unmapped']} unmapped "
          f"filetypes, {dropped['no-command']} without a command")
    if missing:
        print(f"\n{len(missing)} not yet carried:")
        for server in missing[:30]:
            print(f"  {server['id']}  ({', '.join(server['languages'])})")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
