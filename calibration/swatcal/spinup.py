"""How long a spin-up a project needs, and whether one spin-up can serve every run of a calibration
(`docs/spinup.md`).

    python -m swatcal.spinup --exe swatplus-rust --project <TxtInOut> --out spin

Part A, initial state: three copies of the project start from a dry, the usual and a wet state
(soil water as a share of field capacity, `parameters.bsn`'s `sw_init`; the aquifers' water table,
`aquifer.aqu`'s `dep_wt`) and print every day from the first. Year by year, how far apart are the
outlet's flow and the basin's stores? Once the three agree, the start no longer matters: that year
is the spin-up the project needs.

Part B, shared spin-up: parameter sets (a Latin hypercube over the space of `swatcal.compare`) run
twice, each with its own warm-up and all from one warm-up made with the project's parameters
(`swatplus-rust batch --shared-spinup`). How far apart are their flows, and does the shared
spin-up change which sets score best against a synthetic truth?
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import time

import numpy as np

from . import metrics
from .compare import SPACE, TRUTH
from .params import lhs
from .runner import run_sets

STATES = {
    # name: (sw_init, the water table as a share of the aquifer's depth, 0 = full)
    "dry": (0.05, 0.95),
    "usual": (None, None),
    "wet": (1.0, 0.05),
}


def _rewrite_table(path, edit):
    """Edits a whitespace table in place: `edit(header, row_values)` returns the new values."""
    lines = open(path).read().splitlines()
    head = lines[1].split()
    for i in range(2, len(lines)):
        v = lines[i].split()
        if len(v) == len(head):
            lines[i] = "  ".join(edit(head, v))
    open(path, "w").write("\n".join(lines) + "\n")


def state_copy(project: str, dest: str, sw_init, wt_share) -> str:
    """A copy of the project that starts from the given state and prints from the first day, with
    the basin's water balance and aquifer each year."""
    if os.path.exists(dest):
        shutil.rmtree(dest)
    shutil.copytree(project, dest)
    if sw_init is not None:
        def bsn(h, v):
            v[h.index("sw_init")] = f"{sw_init:.5f}"
            return v
        _rewrite_table(os.path.join(dest, "parameters.bsn"), bsn)
    if wt_share is not None:
        def aqu(h, v):
            v[h.index("dep_wt")] = f"{wt_share * float(v[h.index('dep_bot')]):.5f}"
            return v
        _rewrite_table(os.path.join(dest, "aquifer.aqu"), aqu)
    path = os.path.join(dest, "print.prt")
    lines = open(path).read().splitlines()
    f = lines[2].split()
    f[0] = "0"
    lines[2] = "  ".join(f)
    for i, l in enumerate(lines):
        w = l.split()
        if w and w[0] in ("basin_wb", "basin_aqu") and len(w) == 5:
            lines[i] = f"{w[0]:<24}{w[1]:>14}{w[2]:>14}{'y':>14}{w[4]:>14}"
    open(path, "w").write("\n".join(lines) + "\n")
    return dest


def read_table(path):
    """A SWAT+ text table: its columns by name, as strings, one list per column."""
    lines = open(path).read().splitlines()
    head = lines[1].split()
    rows = [l.split() for l in lines[3:] if l.strip()]
    return {h: [r[i] for r in rows] for i, h in enumerate(head)}


def years_of(dates):
    return dates.astype("datetime64[Y]").astype(int) + 1970


def outlet(exe, project, work, threads):
    """The channel with the largest mean outflow."""
    n = sum(1 for l in open(os.path.join(project, "chandeg.con")).read().splitlines()[2:] if l.strip())
    r = run_sets(exe, project, [{}], list(range(1, n + 1)), names=["all"], threads=threads,
                 workdir=os.path.join(work, "outlet"))["all"]
    if isinstance(r, str):
        raise SystemExit(f"the run failed: {r}")
    return max(r.flo, key=lambda c: float(np.mean(r.flo[c])))


def part_a(exe, project, out, channel, threads):
    res = {}
    for name, (sw, wt) in STATES.items():
        d = state_copy(project, os.path.join(out, "states", name), sw, wt)
        r = run_sets(exe, d, [{}], [channel], names=[name], threads=threads,
                     workdir=os.path.join(out, "states", f"{name}-runs"))[name]
        if isinstance(r, str):
            raise SystemExit(f"{name}: {r}")
        tables = os.path.join(out, "states", f"{name}-tables")
        subprocess.run([exe, "run", d, "--out", tables], check=True, capture_output=True)
        wb = read_table(os.path.join(tables, "basin_wb_yr.txt"))
        aq = read_table(os.path.join(tables, "basin_aqu_yr.txt"))
        res[name] = {
            "dates": r.dates, "flo": r.flo[channel],
            "sw": dict(zip(map(int, wb["yr"]), map(float, wb["sw_final"]))),
            "aqu": dict(zip(map(int, aq["yr"]), map(float, aq["stor"]))),
        }
        shutil.rmtree(d, ignore_errors=True)
    dates = res["usual"]["dates"]
    yrs = years_of(dates)
    rows = []
    for y in sorted(set(yrs)):
        k = yrs == y
        q = {n: res[n]["flo"][k] for n in STATES}
        mean_u = float(np.mean(q["usual"]))
        rows.append({
            "year": int(y),
            "flow_spread_pct": 100 * float(np.mean(q["wet"]) - np.mean(q["dry"])) / mean_u,
            "flow_nse_dry_wet": float(metrics.nse(q["dry"], q["wet"])),
            # The low flow (exceeded 90 % of the days): where the aquifer's water shows.
            "low_flow_spread_pct": 100 * float(np.percentile(q["wet"], 10) - np.percentile(q["dry"], 10))
            / float(np.percentile(q["usual"], 10)),
            "sw_dry_usual_wet_mm": [res[n]["sw"].get(int(y), float("nan")) for n in STATES],
            "aqu_dry_usual_wet_mm": [res[n]["aqu"].get(int(y), float("nan")) for n in STATES],
        })
    return rows


def part_b(exe, project, out, channel, threads, n_sets, seed):
    rng = np.random.default_rng(seed)
    xs = np.vstack([np.array(TRUTH), lhs(n_sets, SPACE.lo, SPACE.hi, rng)])
    sets = SPACE.sets(xs)
    names = ["truth"] + [f"s{i:03d}" for i in range(1, n_sets + 1)]
    times = _load_times(out)
    own = _reuse(os.path.join(out, "own", "group00", "out"), names)
    if own is None:
        t0 = time.time()
        own = run_sets(exe, project, sets, [channel], names=names, threads=threads, workdir=os.path.join(out, "own"))
        times["own"] = time.time() - t0
    shared = _reuse(os.path.join(out, "shared", "out"), names)
    if shared is None:
        t0 = time.time()
        shared = _run_shared(exe, project, sets, [channel], names, threads, os.path.join(out, "shared"))
        times["shared"] = time.time() - t0
    with open(os.path.join(out, "times.json"), "w") as f:
        json.dump(times, f)
    t_own, t_shared = times.get("own", float("nan")), times.get("shared", float("nan"))
    q_true = own["truth"].flo[channel]
    yrs = years_of(own["truth"].dates)
    rows = []
    for n in names:
        a, b = own[n], shared[n]
        if isinstance(a, str) or isinstance(b, str):
            rows.append({"set": n, "error": a if isinstance(a, str) else b})
            continue
        qa, qb = a.flo[channel], b.flo[channel]
        rows.append({
            "set": n,
            "kge_shared_vs_own": metrics.kge(qb, qa).kge,
            "pbias_shared_vs_own": float(metrics.pbias(qb, qa)),
            "first_year_pbias": float(metrics.pbias(qb[yrs == yrs[0]], qa[yrs == yrs[0]])),
            "kge_vs_truth_own": metrics.kge(qa, q_true).kge,
            "kge_vs_truth_shared": metrics.kge(qb, q_true).kge,
        })
    ok = [r for r in rows if "error" not in r and r["set"] != "truth"]
    ko = np.array([r["kge_vs_truth_own"] for r in ok])
    ks = np.array([r["kge_vs_truth_shared"] for r in ok])
    rank = float(np.corrcoef(np.argsort(np.argsort(ko)), np.argsort(np.argsort(ks)))[0, 1])
    return {
        "rows": rows,
        "spearman_kge_rank": rank,
        "best_own": ok[int(np.argmax(ko))]["set"],
        "best_shared": ok[int(np.argmax(ks))]["set"],
        "seconds_own": t_own,
        "seconds_shared": t_shared,
        "params": {n: s for n, s in zip(names, sets)},
    }


def _load_times(out):
    try:
        with open(os.path.join(out, "times.json")) as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


def _reuse(folder, names):
    """The series of an earlier run of this part, when all of them are there."""
    from .runner import read_series
    paths = [os.path.join(folder, f"run_{n}.csv") for n in names]
    if not all(os.path.exists(p) for p in paths):
        return None
    return {n: read_series(p) for n, p in zip(names, paths)}


def _run_shared(exe, project, sets, channels, names, threads, work):
    """`run_sets` with `--shared-spinup` (one group: every set changes the same parameters)."""
    import csv
    from .runner import read_series
    os.makedirs(work, exist_ok=True)
    keys = sorted(sets[0])
    path = os.path.join(work, "sets.csv")
    with open(path, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["run", *keys])
        for n, s in zip(names, sets):
            w.writerow([n, *[s[k] for k in keys]])
    o = os.path.join(work, "out")
    done = subprocess.run([exe, "batch", project, "--sets", path, "--channels", ",".join(map(str, channels)),
                           "--out", o, "--threads", str(threads), "--shared-spinup"], capture_output=True, text=True)
    if done.returncode not in (0, 3):
        raise SystemExit(done.stderr)
    return {n: read_series(os.path.join(o, f"run_{n}.csv")) for n in names}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--exe", default="swatplus-rust")
    p.add_argument("--project", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--channel", type=int, help="the outlet's channel (default: the largest mean outflow)")
    p.add_argument("--sets", type=int, default=20, help="parameter sets in part B")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--threads", type=int, default=0)
    p.add_argument("--part", choices=("a", "b", "both"), default="both")
    a = p.parse_args(argv)
    os.makedirs(a.out, exist_ok=True)
    channel = a.channel or outlet(a.exe, a.project, a.out, a.threads)
    result = {"project": os.path.basename(os.path.normpath(a.project)), "channel": channel}
    if a.part in ("a", "both"):
        result["initial_state"] = part_a(a.exe, a.project, a.out, channel, a.threads)
    if a.part in ("b", "both"):
        result["shared_spinup"] = part_b(a.exe, a.project, a.out, channel, a.threads, a.sets, a.seed)
    with open(os.path.join(a.out, "spinup.json"), "w") as f:
        json.dump(result, f, indent=1)
    print(json.dumps({k: v for k, v in result.items() if k != "shared_spinup"}, indent=1))
    if "shared_spinup" in result:
        s = result["shared_spinup"]
        print({k: v for k, v in s.items() if k not in ("rows", "params")})


if __name__ == "__main__":
    main()
