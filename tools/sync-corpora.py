#!/usr/bin/env python3
"""Fill in comment syntax and extensions from tokei and scc.

Two independently maintained corpora, both permissively licensed. They are
genuinely independent — on the 187 languages both carry they agree on 77% of
extension sets, 93% of line comments and 89% of block comments, which is far
from the ~100% that would mean one corpus wearing two hats — so agreement
between them is evidence rather than an echo.

The rule follows from that:

  both carry it and agree  -> absorb, corroborated
  only one carries it      -> absorb, single source, as linguist already is
  both carry it and differ -> report it, change nothing

A language that already has comment syntax is never touched. Those entries are
hand-written and a corpus does not get to overrule them.

Extensions are absorbed only when they are plain suffixes. Both corpora list
whole filenames alongside extensions -- `cmakelists.txt` sits in cmake's
extension list -- and langbank keeps the two apart, so anything containing a dot
is reported rather than filed under the wrong taxonomy.

    tools/sync-corpora.py check    report what is missing and what conflicts
    tools/sync-corpora.py create   write the tables and reference them
"""
import argparse
import glob
import json
import re
import sys
import urllib.request

TOKEI = "https://raw.githubusercontent.com/XAMPPRocky/tokei/master/languages.json"
SCC = "https://raw.githubusercontent.com/boyter/scc/master/languages.json"
TABLES = "data/comment-syntax.toml"


def dumps(value):
    """TOML takes literal UTF-8. json.dumps escapes non-BMP characters into
    surrogate pairs, which TOML does not accept -- Mojo's `.🔥` extension is
    the one that finds this out."""
    return json.dumps(value, ensure_ascii=False)


def slug(name):
    out = name.lower().replace("#", "-sharp").replace("++", "pp").replace("*", "-star")
    out = "".join(c if c.isalnum() else "-" for c in out)
    while "--" in out:
        out = out.replace("--", "-")
    return out.strip("-")


def fetch(url):
    return json.loads(urllib.request.urlopen(url, timeout=60).read())


def extensions(raw):
    """id -> extensions, split into unambiguous suffixes and dotted entries."""
    plain, dotted = {}, {}
    for name, entry in raw.items():
        for value in entry.get("extensions", []):
            value = value.lower().lstrip(".")
            target = dotted if "." in value else plain
            target.setdefault(slug(name), set()).add(value)
    return plain, dotted


def carried(languages, lid):
    """Extensions langbank already lists for a language."""
    _, text, _ = languages[lid]
    match = re.search(r"^extensions = \[(.*?)\]\n", text, re.M | re.S)
    return set(re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1))) if match else set()


def corpus(raw, line_key, multi_key, doc_key=None):
    """id -> (line comments, block comment pairs, doc prefixes)."""
    out = {}
    for name, entry in raw.items():
        pairs = tuple(
            sorted(
                tuple(pair)
                for pair in (entry.get(multi_key) or [])
                if isinstance(pair, (list, tuple)) and len(pair) == 2
            )
        )
        line = tuple(sorted(entry.get(line_key) or []))
        docs = tuple(sorted(entry.get(doc_key) or [])) if doc_key else ()
        if line or pairs:
            out[slug(name)] = (line, pairs, docs)
    return out


def local():
    """id -> (path, text, has_comments)."""
    out = {}
    for path in sorted(glob.glob("data/languages/*.toml")):
        text = open(path).read()
        lid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        out[lid] = (path, text, bool(re.search(r"^comments = ", text, re.M)))
    return out


def existing_tables():
    """name -> (line, block, docs), for the tables already written."""
    out = {}
    text = open(TABLES).read()
    for block in re.split(r"\n\[", text)[1:]:
        name = block.split("]")[0]

        def arr(key):
            match = re.search(rf"^{key} = \[(.*?)\]", block, re.M)
            return re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1)) if match else []

        pairs = re.findall(r'\["((?:[^"\\]|\\.)*)", "((?:[^"\\]|\\.)*)"\]', block)
        out[name] = (tuple(sorted(arr("line"))), tuple(sorted(pairs)), tuple(sorted(arr("documentation"))))
    return out


def reconcile(tokei, scc, languages):
    """id -> syntax, plus the conflicts nobody should guess at."""
    resolved, conflicts = {}, {}
    for lid, (_, _, has) in languages.items():
        if has:
            continue
        left, right = tokei.get(lid), scc.get(lid)
        if left and right:
            # documentation prefixes only exist in tokei, so compare the rest
            if left[:2] == right[:2]:
                resolved[lid] = left
            else:
                conflicts[lid] = (left, right)
        elif left or right:
            resolved[lid] = left or right
    return resolved, conflicts


def render(name, syntax):
    line, pairs, docs = syntax
    blocks = ", ".join(f"[{dumps(a)}, {dumps(b)}]" for a, b in pairs)
    return (
        f"\n[{name}]\n"
        f"line = {dumps(list(line))}\n"
        f"block = [{blocks}]\n"
        f"documentation = {dumps(list(docs))}\n"
        f"quotes = []\n"
        f"multi-quotes = []\n"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    args = parser.parse_args()

    tokei_raw = (lambda raw: raw.get("languages", raw))(fetch(TOKEI))
    scc_raw = fetch(SCC)
    tokei = corpus(tokei_raw, "line_comment", "multi_line_comments", "important_syntax")
    scc = corpus(scc_raw, "line_comment", "multi_line")
    languages = local()
    resolved, conflicts = reconcile(tokei, scc, languages)

    plain, dotted = {}, {}
    for raw in (tokei_raw, scc_raw):
        for source, target in zip(extensions(raw), (plain, dotted)):
            for lid, values in source.items():
                target.setdefault(lid, set()).update(values)
    missing_ext = {
        lid: sorted(values - carried(languages, lid))
        for lid, values in plain.items()
        if lid in languages and values - carried(languages, lid)
    }
    refused = {
        lid: sorted(values - carried(languages, lid))
        for lid, values in dotted.items()
        if lid in languages and values - carried(languages, lid)
    }

    if args.command == "check":
        have = sum(1 for _, _, has in languages.values() if has)
        print(f"comment syntax: {have} of {len(languages)} languages")
        print(f"  available from tokei/scc and not yet carried: {len(resolved)}")
        print(f"  the two corpora disagree, left alone: {len(conflicts)}")
        for lid, (left, right) in sorted(conflicts.items()):
            print(f"    {lid}: tokei line={list(left[0])} block={len(left[1])}"
                  f" | scc line={list(right[0])} block={len(right[1])}")
        total = sum(len(v) for v in missing_ext.values())
        print(f"\nextensions not yet carried: {total} across {len(missing_ext)} languages")
        if refused:
            print(f"  refused as ambiguous — a dot means a filename, not a suffix: "
                  f"{sum(len(v) for v in refused.values())}")
            for lid, values in sorted(refused.items()):
                print(f"    {lid}: {' '.join(values)}")
        return 1 if resolved or missing_ext else 0

    tables = existing_tables()
    by_syntax = {syntax: name for name, syntax in tables.items()}
    appended = []
    for lid in sorted(resolved):
        syntax = resolved[lid]
        name = by_syntax.get(syntax)
        if name is None:
            # a table is named after the first language that needs it, which is
            # the convention the hand-written ones already follow
            name = lid
            by_syntax[syntax] = name
            appended.append((name, syntax))
        path, text, _ = languages[lid]
        anchor = re.search(r"^(extensions|filenames|shebangs) = \[.*?\]\n", text, re.M | re.S)
        insert = anchor.end() if anchor else re.search(r"^role = .*\n", text, re.M).end()
        open(path, "w").write(text[:insert] + f"comments = {dumps(name)}\n" + text[insert:])

    if appended:
        with open(TABLES, "a") as handle:
            for name, syntax in appended:
                handle.write(render(name, syntax))
    fresh = local()
    for lid, values in sorted(missing_ext.items()):
        path, text, _ = fresh[lid]
        match = re.search(r"^extensions = \[(.*?)\]\n", text, re.M | re.S)
        have = set(re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1))) if match else set()
        line = "extensions = " + dumps(sorted(have | set(values))) + "\n"
        if match:
            text = text[: match.start()] + line + text[match.end():]
        else:
            role = re.search(r"^role = .*\n", text, re.M)
            text = text[: role.end()] + line + text[role.end():]
        open(path, "w").write(text)

    print(f"comment syntax written for {len(resolved)} languages, "
          f"{len(appended)} new shared tables, {len(conflicts)} conflicts left alone")
    print(f"extensions added: {sum(len(v) for v in missing_ext.values())} across "
          f"{len(missing_ext)} languages; {sum(len(v) for v in refused.values())} refused as ambiguous")
    return 0


if __name__ == "__main__":
    sys.exit(main())
