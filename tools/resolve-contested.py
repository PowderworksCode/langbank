#!/usr/bin/env python3
"""Settle contested extensions where independent corpora agree.

An extension claimed by several languages resolves to nothing unless exactly
one claimant declares it primary. 160 are in that state, which is honest and
unhelpful, and some of them are only unresolved because nobody has looked.

tokei and scc are independently maintained -- measured at 77%/93%/89% agreement
on the languages both carry, far from the ~100% a shared lineage would show --
so where both name the same claimant that is corroboration rather than an echo.
Only those are applied. Where one corpus has an opinion and the other is silent,
or where the two disagree, the extension is reported and left alone: a primary
claim is a decision about what a file *is*, and one source is not enough to make
it on langbank's behalf.

    tools/resolve-contested.py check
    tools/resolve-contested.py create
"""
import argparse
import collections
import glob
import json
import re
import sys
import urllib.request

TOKEI = "https://raw.githubusercontent.com/XAMPPRocky/tokei/master/languages.json"
SCC = "https://raw.githubusercontent.com/boyter/scc/master/languages.json"


def slug(name):
    out = name.lower().replace("#", "-sharp").replace("++", "pp").replace("*", "-star")
    out = "".join(c if c.isalnum() else "-" for c in out)
    while "--" in out:
        out = out.replace("--", "-")
    return out.strip("-")


def fetch(url):
    return json.loads(urllib.request.urlopen(url, timeout=90).read())


def owners(corpus):
    out = collections.defaultdict(set)
    for name, entry in corpus.items():
        for extension in entry.get("extensions", []):
            out[extension.lower().lstrip(".")].add(slug(name))
    return out


def local():
    claims, primary, paths = collections.defaultdict(list), {}, {}
    for path in sorted(glob.glob("data/languages/*.toml")):
        text = open(path).read()
        lid = re.search(r'^id = "([^"]+)"', text, re.M).group(1)
        paths[lid] = path

        def field(name):
            match = re.search(rf"^{name} = \[(.*?)\]\n", text, re.M | re.S)
            return re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1)) if match else []

        for extension in field("extensions"):
            claims[extension].append(lid)
        for extension in field("primary-extensions"):
            primary[extension] = lid
    return claims, primary, paths


def evidence():
    tokei = fetch(TOKEI)
    return owners(tokei.get("languages", tokei)), owners(fetch(SCC))


def classify(claims, primary, tokei, scc):
    agreed, single, disputed, unhelped = {}, {}, {}, []
    for extension, claimants in claims.items():
        if len(claimants) < 2 or extension in primary:
            continue
        left = {c for c in tokei.get(extension, ()) if c in claimants}
        right = {c for c in scc.get(extension, ()) if c in claimants}
        if len(left) == 1 and left == right:
            agreed[extension] = next(iter(left))
        elif len(left) == 1 and not right:
            single[extension] = next(iter(left))
        elif len(right) == 1 and not left:
            single[extension] = next(iter(right))
        elif left and right and left != right:
            disputed[extension] = (sorted(left), sorted(right))
        else:
            unhelped.append(extension)
    return agreed, single, disputed, unhelped


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check", "create"])
    args = parser.parse_args()

    claims, primary, paths = local()
    tokei, scc = evidence()
    agreed, single, disputed, unhelped = classify(claims, primary, tokei, scc)

    if args.command == "check":
        print(f"contested extensions resolving to nothing: "
              f"{len(agreed) + len(single) + len(disputed) + len(unhelped)}")
        print(f"  both corpora agree, and can be applied : {len(agreed)}")
        print(f"  one corpus only, left for a person     : {len(single)}")
        print(f"  the corpora disagree, left alone       : {len(disputed)}")
        for extension, (left, right) in sorted(disputed.items()):
            print(f"    .{extension}: tokei {left} scc {right}")
        print(f"  no corpus has an opinion               : {len(unhelped)}")
        return 1 if agreed else 0

    by_language = collections.defaultdict(list)
    for extension, language in agreed.items():
        by_language[language].append(extension)
    for language, extensions in sorted(by_language.items()):
        path = paths[language]
        text = open(path).read()
        match = re.search(r"^primary-extensions = \[(.*?)\]\n", text, re.M | re.S)
        have = re.findall(r'"((?:[^"\\]|\\.)*)"', match.group(1)) if match else []
        line = "primary-extensions = " + json.dumps(sorted(set(have) | set(extensions)),
                                                    ensure_ascii=False) + "\n"
        if match:
            text = text[: match.start()] + line + text[match.end():]
        else:
            anchor = re.search(r"^extensions = \[.*?\]\n", text, re.M | re.S)
            text = text[: anchor.end()] + line + text[anchor.end():]
        open(path, "w").write(text)
    rows = []
    for extension, language in sorted(single.items()):
        rows.append(f'\n[[gap]]\nsubject = {json.dumps(extension)}\nreason = "uncorroborated"\n'
                    f'note = {json.dumps(f"one corpus names {language}; the other is silent")}\n')
    for extension, (left, right) in sorted(disputed.items()):
        rows.append(f'\n[[gap]]\nsubject = {json.dumps(extension)}\nreason = "sources-disagree"\n'
                    f'note = {json.dumps(f"tokei says {left}, scc says {right}")}\n')
    open("data/gaps/extension-owner.toml", "w").write(
        "# Contested extensions no corroborated source settles. Detection\n"
        "# declines rather than guessing; these say why.\n"
        'facet = "extension-owner"\n' + "".join(rows)
    )

    print(f"settled {len(agreed)} contested extensions across {len(by_language)} languages; "
          f"{len(single)} single-source and {len(disputed)} disputed left alone")
    return 0


if __name__ == "__main__":
    sys.exit(main())
