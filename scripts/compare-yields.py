#!/usr/bin/env python3
"""Compares SWAT+'s crop yield tables with the port's: crop_yld_aa.txt (by HRU and plant) and
basin_crop_yld_yr.txt / _aa.txt (by year and plant). A value matches within 1e-4 relative or one
unit in the last printed decimal. Exit 1 on a mismatch.

usage: scripts/compare-yields.py <SWAT+ folder> <port folder>
"""
import os
import sys


def rows(path, skip):
    out = []
    with open(path) as f:
        for i, line in enumerate(f):
            if i < skip:
                continue
            t = line.split()
            if t:
                out.append(t)
    return out


def close(a, b, unit):
    x, y = float(a), float(b)
    d = abs(x - y)
    return d <= unit * (1 + 1e-6) or d <= 1e-4 * max(abs(x), abs(y))


def main():
    ref, port = sys.argv[1], sys.argv[2]
    ok = True
    for name, skip, key, vals, unit in [
        ("crop_yld_aa.txt", 3, (3, 4, 5), (6, 7, 8, 9), 0.001),
        ("basin_crop_yld_yr.txt", 2, (0, 1, 2), (3, 4, 5), 1e-6),
        ("basin_crop_yld_aa.txt", 2, (0, 1, 2), (3, 4, 5), 1e-6),
    ]:
        a, b = os.path.join(ref, name), os.path.join(port, name)
        if not os.path.exists(a):
            continue
        ra = rows(a, skip)
        if not os.path.exists(b):
            # The port writes a table on its first row; with no crops, SWAT+'s has a header only.
            if ra:
                print(f"{name}: missing from the port")
                ok = False
            continue
        rb = rows(b, skip)
        bad = 0
        if len(ra) != len(rb):
            print(f"{name}: {len(ra)} rows vs {len(rb)}")
            ok = False
        for x, y in zip(ra, rb):
            if [x[k] for k in key] != [y[k] for k in key] or not all(close(x[k], y[k], unit) for k in vals):
                if bad < 3:
                    print(f"{name}: SWAT+ {' '.join(x)} | port {' '.join(y)}")
                bad += 1
        print(f"{name}: {len(ra)} rows, {bad} mismatches")
        ok &= bad == 0
    sys.exit(0 if ok else 1)


main()
