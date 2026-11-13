"""Running parameter sets through `swatplus-rust batch` and reading the series back."""

from __future__ import annotations

import csv
import os
import subprocess
import tempfile
from dataclasses import dataclass

import numpy as np


@dataclass
class RunSeries:
    dates: np.ndarray  # datetime64[D]
    precip: np.ndarray  # mm, basin mean
    et: np.ndarray  # mm, basin mean
    flo: dict[int, np.ndarray]  # channel -> m3/s
    dep: dict[int, np.ndarray]  # channel -> m, SWAT+'s rating curve (when asked for)
    dep_mann: dict[int, np.ndarray]


def read_series(path: str) -> RunSeries:
    with open(path, newline="") as f:
        rows = list(csv.reader(f))
    head, body = rows[0], rows[1:]
    col = {h: i for i, h in enumerate(head)}

    def num(name):
        return np.array([float(r[col[name]]) for r in body])

    def by_channel(prefix):
        return {int(h[len(prefix):]): num(h) for h in head if h.startswith(prefix) and h[len(prefix):].isdigit()}

    return RunSeries(
        dates=np.array([r[col["date"]] for r in body], dtype="datetime64[D]"),
        precip=num("precip_mm"),
        et=num("et_mm"),
        flo=by_channel("flo_"),
        dep=by_channel("dep_"),
        dep_mann=by_channel("dep_mann_"),
    )


def run_sets(exe: str, project: str, sets: list[dict[str, float]], channels: list[int], names=None, depth=False,
             threads=0, workdir=None, cal_parms=None) -> dict[str, RunSeries | str]:
    """Run each parameter set ({"cn2:pctchg": -10, ...}; {} is the project as it is) and return
    each run's series by name, or the error message of a failed run.

    A parameter a set doesn't name is not touched: sets are run in groups that change the same
    parameters. ("pctchg 0" is not the same as no change: SWAT+ holds every changed value to
    cal_parms.cal's range, so a curve number of 98 changed by 0 % becomes 95.)
    """
    names = names or [f"{k + 1:04d}" for k in range(len(sets))]
    if len(names) != len(sets):
        raise ValueError("one name per set")
    for s in sets:
        for k in s:
            if ":" not in k or k.split(":")[1] not in ("absval", "abschg", "pctchg", "relchg"):
                raise ValueError(f"{k}: write it as parameter:change, the change absval, abschg, pctchg or relchg")
    work = workdir or tempfile.mkdtemp(prefix="swatcal-")
    groups: dict[tuple[str, ...], list[int]] = {}
    for i, s in enumerate(sets):
        groups.setdefault(tuple(sorted(s)), []).append(i)
    result: dict[str, RunSeries | str] = {}
    for g, (keys, idx) in enumerate(sorted(groups.items())):
        gdir = os.path.join(work, f"group{g:02d}")
        os.makedirs(gdir, exist_ok=True)
        sets_path = os.path.join(gdir, "sets.csv")
        with open(sets_path, "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["run", *keys])
            for i in idx:
                w.writerow([names[i], *[sets[i][k] for k in keys]])
        out = os.path.join(gdir, "out")
        cmd = [exe, "batch", project, "--sets", sets_path, "--channels", ",".join(map(str, channels)), "--out", out,
               "--threads", str(threads)]
        if depth:
            cmd.append("--depth")
        if cal_parms:
            cmd += ["--cal-parms", cal_parms]
        done = subprocess.run(cmd, check=False, capture_output=True, text=True)
        status = {}
        runs_csv = os.path.join(out, "runs.csv")
        if os.path.exists(runs_csv):
            with open(runs_csv, newline="") as f:
                for r in csv.reader(f):
                    if r and r[0] != "run":
                        status[r[0]] = r[1]
        for i in idx:
            n = names[i]
            st = status.get(n, done.stderr.strip().splitlines()[-1] if done.stderr.strip() else "not run")
            result[n] = read_series(os.path.join(out, f"run_{n}.csv")) if st == "ok" else st
    return result
