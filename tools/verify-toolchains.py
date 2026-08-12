#!/usr/bin/env python3
"""Run every version probe langbank states, on whatever is installed here.

Langbank supplies the facts and never executes anything, so nothing in the
crate can tell whether `pattern` actually matches what the program prints. This
does: it runs each probe it can, applies the stated pattern to the stated
stream, and reports. Programs that are absent are skipped, not failed — no
machine has all of them.

    tools/verify-toolchains.py          verify what is installed
    tools/verify-toolchains.py --strict fail if an installed probe does not match
"""
import argparse
import glob
import re
import shutil
import subprocess
import sys

import tomllib


def probes():
    for path in sorted(glob.glob("data/toolchains/*.toml")):
        with open(path, "rb") as handle:
            entry = tomllib.load(handle)
        version = entry.get("version")
        if version:
            yield entry["id"], entry.get("programs", []), version


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    verified = skipped = failed = 0
    for tid, programs, version in probes():
        program = next((p for p in programs if shutil.which(p)), None)
        if program is None:
            print(f"  {tid:10} skipped   (none of {' '.join(programs)} installed)")
            skipped += 1
            continue
        try:
            result = subprocess.run(
                [program, *version["arguments"]], capture_output=True, text=True, timeout=30
            )
        except Exception as error:  # noqa: BLE001 - reported, not raised
            print(f"  {tid:10} FAILED    running {program}: {error}")
            failed += 1
            continue
        stream = result.stdout if version.get("stream", "stdout") == "stdout" else result.stderr
        line = next((l for l in stream.splitlines() if l.strip()), "")
        match = re.search(version["pattern"], line)
        if match:
            print(f"  {tid:10} verified  {match.group(1):12} via {program}")
            verified += 1
        else:
            print(f"  {tid:10} FAILED    pattern did not match {line[:60]!r}")
            failed += 1

    print(f"\n{verified} verified, {skipped} skipped, {failed} failed")
    return 1 if failed and args.strict else 0


if __name__ == "__main__":
    sys.exit(main())
