#!/usr/bin/env python3
"""Summarizes scripts/bench.sh's runs.csv as a Markdown table: per scenario and engine the
median and range of the wall time, the peak memory and what was written, and the speed-up."""
import csv
import statistics
import sys

rows = list(csv.DictReader(open(sys.argv[1])))
machine = open(sys.argv[2]).read() if len(sys.argv) > 2 else ""
by = {}
for r in rows:
    by.setdefault((r["scenario"], r["engine"]), []).append(r)

print("# swatplus-rust vs SWAT+ Fortran\n")
print("```text\n" + machine.strip() + "\n```\n")
print("| Scenario | Engine | Runs | Wall time, median (s) | Min–max (s) | Peak memory (MB) | Files written | Written (MB) |")
print("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |")
speed = []
for scen in dict.fromkeys(r["scenario"] for r in rows):
    med = {}
    for eng in ("fortran", "rust"):
        rs = by.get((scen, eng), [])
        if not rs:
            continue
        w = [float(r["wall_s"]) for r in rs]
        m = statistics.median(w)
        med[eng] = m
        rss = statistics.median(float(r["max_rss_kb"]) for r in rs) / 1024
        print(f"| {scen} | {'SWAT+ Fortran' if eng == 'fortran' else 'swatplus-rust'} | {len(w)} | {m:.2f} | {min(w):.2f}–{max(w):.2f} | {rss:.0f} | {rs[0]['files']} | {int(rs[0]['bytes']) / 1e6:.1f} |")
    if len(med) == 2 and med["rust"] > 0:
        speed.append((scen, med["fortran"] / med["rust"]))
print()
for scen, x in speed:
    print(f"- **{scen}**: swatplus-rust takes {1 / x:.2f} of SWAT+ Fortran's time ({x:.2f}× as fast).")
if len(sys.argv) > 3:
    agree = list(csv.DictReader(open(sys.argv[3])))
    print("\n## Agreement of the tables\n")
    print("Each table the port wrote, against SWAT+'s from the same run: identical (every value the same number), within decision K12's tolerance, or mismatch; and the largest relative difference of any value.\n")
    print("| Scenario | Tables | Identical | Within tolerance | Mismatch | Largest relative difference |")
    print("| --- | ---: | ---: | ---: | ---: | ---: |")
    for scen in dict.fromkeys(a["scenario"] for a in agree):
        ag = [a for a in agree if a["scenario"] == scen]
        rs = [a["result"] for a in ag]
        worst = max((float(a.get("max_rel_diff") or 0) for a in ag if a.get("max_rel_diff") not in (None, "", "nan")), default=0.0)
        print(f"| {scen} | {len(rs)} | {rs.count('identical')} | {rs.count('ok')} | {rs.count('mismatch')} | {worst:.2g} |")
    bad = [f"{a['scenario']} {a['table']}" for a in agree if a["result"] == "mismatch"]
    if bad:
        print("\nMismatches: " + ", ".join(bad) + " (the .cmp files say where).")
print("\nSWAT+ Fortran writes more files than the port (its other outputs and logs); the columns of files written show how much.")
