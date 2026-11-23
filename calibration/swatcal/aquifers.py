"""The water balance of a project's aquifers, year by year (`docs/spinup.md`, "The aquifers").

    python -m swatcal.aquifers --exe swatplus-rust --project <TxtInOut> --out aq

Runs a copy of the project that prints every aquifer each year from the first (`nyskip` 0), and
reports for the shallow aquifers and the deep ones (`aquifer.con` names starting `aqu_deep`, which
the SWAT+ editor makes to take the shallow aquifers' deep seepage): how their storage changed, how
much recharge, return flow, seepage and revap they had, and whether their return flow goes
anywhere (`aquifer.con`'s `out_tot`).
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess

import numpy as np

from .spinup import read_table, state_copy


def deep_names(project):
    """The deep aquifers, and whether each sends its flow anywhere (`out_tot` > 0)."""
    lines = open(os.path.join(project, "aquifer.con")).read().splitlines()
    head = lines[1].split()
    out = {}
    for l in lines[2:]:
        v = l.split()
        if len(v) > head.index("out_tot") and v[1].startswith("aqu_deep"):
            out[v[1]] = int(v[head.index("out_tot")]) > 0
    return out


def balance(exe, project, out):
    d = state_copy(project, os.path.join(out, "copy"), None, None)
    path = os.path.join(d, "print.prt")
    lines = open(path).read().splitlines()
    for i, l in enumerate(lines):
        w = l.split()
        if w and w[0] == "aquifer" and len(w) == 5:
            lines[i] = f"{w[0]:<24}{w[1]:>14}{w[2]:>14}{'y':>14}{w[4]:>14}"
    open(path, "w").write("\n".join(lines) + "\n")
    tables = os.path.join(out, "tables")
    subprocess.run([exe, "run", d, "--out", tables], check=True, capture_output=True)
    t = read_table(os.path.join(tables, "aquifer_yr.txt"))
    deep = deep_names(project)
    shutil.rmtree(d, ignore_errors=True)
    yr = np.array(t["yr"], int)
    name = np.array(t["name"])
    col = {k: np.array(t[k], float) for k in ("stor", "rchrg", "flo", "seep", "revap", "dep_wt")}
    y0, y1 = int(yr.min()), int(yr.max())

    def group(names):
        k0 = np.isin(name, names) & (yr == y0)
        k1 = np.isin(name, names) & (yr == y1)
        ka = np.isin(name, names)
        change = col["stor"][k1] - col["stor"][k0]
        years = y1 - y0 + 1
        return {
            "aquifers": len(names),
            "storage_change_mm": {"min": float(change.min()), "median": float(np.median(change)), "max": float(change.max())},
            # Means over the aquifers of their yearly sums, mm a year.
            **{f"{k}_mm_per_year": float(col[k][ka].sum() / len(names) / years) for k in ("rchrg", "flo", "seep", "revap")},
            "depth_to_water_last_year_m": [float(v) for v in col["dep_wt"][k1]] if len(names) <= 10 else None,
        }

    names = sorted(set(name))
    shallow = [n for n in names if n not in deep]
    return {
        "years": [y0, y1],
        "shallow": group(shallow),
        "deep": group(sorted(deep)) if deep else None,
        "deep_flow_goes_somewhere": deep,
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--exe", default="swatplus-rust")
    p.add_argument("--project", required=True)
    p.add_argument("--out", required=True)
    a = p.parse_args(argv)
    os.makedirs(a.out, exist_ok=True)
    r = balance(a.exe, a.project, a.out)
    with open(os.path.join(a.out, "aquifers.json"), "w") as f:
        json.dump(r, f, indent=1)
    print(json.dumps(r, indent=1))


if __name__ == "__main__":
    main()
