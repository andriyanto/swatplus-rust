#!/usr/bin/env python3
"""Summarises the comparisons of a validation run: the `.cmp` files that `swatplus-rust compare`
and `scripts/compare-yields.py` wrote, one folder per scenario.

usage: scripts/validation-summary.py <folder with one subfolder per scenario> [header text]

Writes Markdown to stdout: per scenario, the tables and columns compared, the values, the
mismatches and the largest absolute and relative differences. The .cmp files hold only counts and
differences, never the values themselves.
"""

import os
import re
import sys

COL = re.compile(r"^\s+(ok|MISMATCH)\s+(\S+)\s+values\s+(\d+)\s+mismatches\s+(\d+)\s+max abs (\S+)\s+max rel (\S+)")
YLD = re.compile(r"^(\S+): (\d+) rows, (\d+) mismatches")


def scenario(path):
    s = dict(tables=0, columns=0, values=0, mismatches=0, abs=0.0, rel=0.0, ytables=0, yrows=0, ymis=0, problems=[])
    for f in sorted(os.listdir(path)):
        p = os.path.join(path, f)
        if f == "yields.cmp":
            for line in open(p):
                m = YLD.match(line)
                if m:
                    s["ytables"] += 1
                    s["yrows"] += int(m.group(2))
                    s["ymis"] += int(m.group(3))
                elif line.strip():
                    s["problems"].append(f"{f}: {line.strip()}")
            continue
        if not f.endswith(".cmp"):
            continue
        s["tables"] += 1
        cols = 0
        for line in open(p):
            m = COL.match(line)
            if m:
                cols += 1
                s["values"] += int(m.group(3))
                s["mismatches"] += int(m.group(4))
                s["abs"] = max(s["abs"], float(m.group(5)))
                s["rel"] = max(s["rel"], float(m.group(6)))
            elif "missing" in line or "not in" in line:
                s["problems"].append(f"{f}: {line.strip()}")
        if cols == 0:
            s["problems"].append(f"{f}: no column compared")
        s["columns"] += cols
    return s


def main():
    root = sys.argv[1]
    header = sys.argv[2] if len(sys.argv) > 2 else ""
    names = sorted(d for d in os.listdir(root) if os.path.isdir(os.path.join(root, d)))
    rows = [(n, scenario(os.path.join(root, n))) for n in names]
    print("# Validation against SWAT+ 6f51536\n")
    if header:
        print(header + "\n")
    print("| Scenario | Tables | Columns | Values | Mismatches | Largest abs. difference | Largest rel. difference | Crop yield rows (mismatches) |")
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |")
    t = dict(tables=0, columns=0, values=0, mismatches=0, ymis=0, yrows=0)
    for n, s in rows:
        y = f"{s['yrows']:,} ({s['ymis']})" if s["ytables"] else "none"
        print(f"| {n} | {s['tables']} | {s['columns']:,} | {s['values']:,} | {s['mismatches']} | {s['abs']:.4f} | {s['rel']:.2e} | {y} |")
        for k in t:
            t[k] += s[k]
    print(f"| **All {len(rows)}** | {t['tables']} | {t['columns']:,} | {t['values']:,} | {t['mismatches']} | | | {t['yrows']:,} ({t['ymis']}) |")
    problems = [p for _, s in rows for p in s["problems"]]
    print()
    if problems:
        print("Problems:\n")
        for p in problems:
            print(f"- {p}")
    else:
        print("No table has a missing row, an extra row or a column not compared.")
    print("\nA value matches within 1e-4 relative or one unit in the last printed decimal (decision K12);")
    print("the largest differences are of all values, matched or not. Each scenario's `.cmp` files are")
    print("in its folder.")


main()
