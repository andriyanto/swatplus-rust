#!/usr/bin/env python3
"""How well the two engines' tables agree after scripts/bench.sh: each table the port wrote
against SWAT+'s from the same scenario's last runs. A table is `identical` when every value
reads as the same number (the column widths and number formats may differ), `ok` when it
matches within decision K12's tolerance (`swatplus-rust compare` for the daily-keyed tables,
the rule of scripts/compare-yields.py for the crop yields), else `mismatch`. Also the largest
relative difference of any value, which shows the last-bit drift (FMA on ARM64).

usage: scripts/bench-agree.py <results folder> [work folder] [swatplus-rust]
       (writes agreement.csv there and rewrites summary.md)
"""
import csv
import glob
import math
import os
import subprocess
import sys

res = sys.argv[1]
work = sys.argv[2] if len(sys.argv) > 2 else os.path.expanduser("~/swat-bench")
here = os.path.dirname(os.path.abspath(__file__))
rust = sys.argv[3] if len(sys.argv) > 3 else os.path.join(here, "..", "target", "release", "swatplus-rust")
scenarios = list(dict.fromkeys(r["scenario"] for r in csv.DictReader(open(os.path.join(res, "runs.csv")))))
CROP = {"crop_yld_aa.txt": (3, 3), "basin_crop_yld_yr.txt": (2, 3), "basin_crop_yld_aa.txt": (2, 3)}


def num(t):
    try:
        v = float(t)
    except ValueError:
        return None
    return v


def unit(t):
    """One unit in the last printed decimal of `t` (0 for an exponent form)."""
    if "e" in t.lower() or "." not in t:
        return 0.0
    return 10.0 ** -len(t.split(".")[1])


def data(path, skip):
    with open(path, errors="replace") as f:
        return [l.split() for l in f.readlines()[skip:] if l.split()]


def drift(a, b):
    """(all values equal, largest relative difference) of two aligned row lists."""
    same, worst = True, 0.0
    for ra, rb in zip(a, b):
        for x, y in zip(ra, rb):
            vx, vy = num(x), num(y)
            if vx is None or vy is None:
                continue
            if math.isnan(vx) and math.isnan(vy):
                continue
            if vx != vy:
                same = False
                d = abs(vx - vy) / max(abs(vx), abs(vy))
                worst = max(worst, d if not math.isnan(d) else 1.0)
    return same and len(a) == len(b), worst


rows = []
for s in scenarios:
    port_dir, ref_dir = os.path.join(work, f"{s}-rust", "out"), os.path.join(work, f"{s}-fortran")
    for t in sorted(glob.glob(os.path.join(port_dir, "*.txt"))):
        name = os.path.basename(t)
        f = os.path.join(ref_dir, name)
        if not os.path.exists(f):
            continue
        if name in CROP:
            skip, nkey = CROP[name]
            a, b = data(f, skip), data(t, skip)
            ka = {tuple(r[:nkey]): r for r in a}
            kb = {tuple(r[:nkey]): r for r in b}
            ok = ka.keys() == kb.keys() and all(
                abs(num(x) - num(y)) <= max(unit(x), unit(y), 1e-6) * (1 + 1e-6) or abs(num(x) - num(y)) <= 1e-4 * max(abs(num(x)), abs(num(y)))
                for k in ka for x, y in zip(ka[k][nkey:], kb[k][nkey:]) if num(x) is not None and num(y) is not None)
            same, worst = drift([ka[k] for k in sorted(ka)], [kb.get(k, []) for k in sorted(ka)])
        else:
            cmp_out = os.path.join(res, f"{s}-{name[:-4]}.cmp")
            with open(cmp_out, "w") as o:
                ok = subprocess.call([rust, "compare", f, t], stdout=o, stderr=subprocess.STDOUT) == 0
            # Title, column names, units: then the rows, in the same order when both have them all.
            a, b = data(f, 3), data(t, 3)
            same, worst = drift(a, b) if len(a) == len(b) else (False, float("nan"))
        result = "identical" if same and ok else ("ok" if ok else "mismatch")
        rows.append({"scenario": s, "table": name[:-4], "result": result, "max_rel_diff": f"{worst:.3g}"})

with open(os.path.join(res, "agreement.csv"), "w", newline="") as o:
    w = csv.DictWriter(o, fieldnames=["scenario", "table", "result", "max_rel_diff"])
    w.writeheader()
    w.writerows(rows)
with open(os.path.join(res, "summary.md"), "w") as o:
    subprocess.call([sys.executable, os.path.join(here, "bench-summary.py"), os.path.join(res, "runs.csv"),
                     os.path.join(res, "machine.txt"), os.path.join(res, "agreement.csv")], stdout=o)
print(open(os.path.join(res, "summary.md")).read())
