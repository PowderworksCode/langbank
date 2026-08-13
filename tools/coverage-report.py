#!/usr/bin/env python3
"""How much does langbank actually know about each language?

Counts what is present per language across everything langbank models, so the
gaps are visible as a distribution rather than as a feeling. Detection is the
floor — a language nothing can recognise is not carried at all — and everything
above it is optional and mostly absent, which is the honest state of a registry
that absorbed breadth before depth.

    tools/coverage-report.py            summary and the shape of the gaps
    tools/coverage-report.py --detail   one line per language
    tools/coverage-report.py --gaps FACET   list languages missing one facet
"""
import argparse
import collections
import glob
import re
import sys

FACETS = [
    "detection",     # anything can recognise a file of this language
    "comments",      # comment syntax
    "facets",        # reusable source surfaces
    "conventions",   # test layout, typecheck, inline tests
    "toolchain",     # any program serves it
    "compiler",      # a compiler or runtime specifically
    "analyser",      # a linter or formatter
    "ecosystem",     # a package manager publishes for it
]


def array(text, key):
    match = re.search(rf"^{key} = \[(.*?)\]\n", text, re.M | re.S)
    return re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1)) if match else []


def languages():
    out = {}
    for path in sorted(glob.glob("data/languages/*.toml")):
        text = open(path).read()
        lid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        out[lid] = {
            "role": (re.search(r'^role = "([^"]+)"', text, re.M) or [None, "?"])[1],
            "detection": bool(array(text, "extensions") or array(text, "filenames")
                              or array(text, "shebangs")),
            "comments": bool(re.search(r"^comments = ", text, re.M)),
            "facets": bool(array(text, "facets")),
            "conventions": "[conventions" in text,
        }
    return out


def toolchains():
    serves, kinds = collections.defaultdict(set), collections.defaultdict(set)
    for path in sorted(glob.glob("data/toolchains/*.toml")):
        text = open(path).read()
        tid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        kind = (re.search(r'^kind = "([^"]+)"', text, re.M) or [None, ""])[1]
        roles = set(array(text, "categories")) | {kind}
        for language in array(text, "languages"):
            serves[language].add(tid)
            kinds[language] |= roles
    return serves, kinds


def ecosystems():
    out = collections.defaultdict(set)
    for path in sorted(glob.glob("data/ecosystems/*.toml")):
        text = open(path).read()
        eid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        for language in array(text, "implied-languages"):
            out[language].add(eid)
    return out


def assess():
    langs, (serves, kinds), ecos = languages(), toolchains(), ecosystems()
    rows = {}
    for lid, facts in langs.items():
        role_kinds = kinds.get(lid, set())
        rows[lid] = {
            "role": facts["role"],
            "detection": facts["detection"],
            "comments": facts["comments"],
            "facets": facts["facets"],
            "conventions": facts["conventions"],
            "toolchain": bool(serves.get(lid)),
            "compiler": bool(role_kinds & {"compiler", "runtime"}),
            "analyser": bool(role_kinds & {"linter", "formatter"}),
            "ecosystem": bool(ecos.get(lid)),
        }
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--detail", action="store_true")
    parser.add_argument("--gaps", choices=FACETS)
    args = parser.parse_args()
    rows = assess()

    if args.gaps:
        missing = sorted(lid for lid, r in rows.items() if not r[args.gaps])
        print(f"{len(missing)} languages have no {args.gaps}:")
        for lid in missing:
            print(f"  {lid}")
        return 0

    if args.detail:
        for lid, r in sorted(rows.items()):
            marks = "".join("x" if r[f] else "." for f in FACETS)
            print(f"  {marks}  {lid}")
        print("  " + "".join(f[0] for f in FACETS) + "  (" + " ".join(FACETS) + ")")
        return 0

    total = len(rows)
    print(f"{total} languages\n")
    print(f"{'facet':14} {'have':>6} {'lack':>6}   share")
    for facet in FACETS:
        have = sum(1 for r in rows.values() if r[facet])
        bar = "#" * round(40 * have / total)
        print(f"{facet:14} {have:6} {total - have:6}   {bar}")

    scores = collections.Counter(sum(1 for f in FACETS if r[f]) for r in rows.values())
    print(f"\nfacets known, by language count:")
    for score in range(len(FACETS) + 1):
        count = scores.get(score, 0)
        if count:
            print(f"  {score} of {len(FACETS)}: {count:4}  {'#' * round(60 * count / total)}")

    by_role = collections.defaultdict(list)
    for lid, r in rows.items():
        by_role[r["role"]].append(sum(1 for f in FACETS if r[f]))
    print(f"\naverage facets known, by role:")
    for role, values in sorted(by_role.items(), key=lambda kv: -len(kv[1])):
        print(f"  {role:14} {len(values):4} languages, {sum(values) / len(values):.1f} of {len(FACETS)}")

    best = sorted(rows.items(), key=lambda kv: -sum(1 for f in FACETS if kv[1][f]))[:8]
    print(f"\nbest known:")
    for lid, r in best:
        print(f"  {sum(1 for f in FACETS if r[f])}/{len(FACETS)}  {lid}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
