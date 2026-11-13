"""The synthetic experiment (M3 in docs/roadmap.md): a world whose answer is known.

    python -m swatcal.synthetic --exe swatplus-rust --project <TxtInOut> --channel 1 --out synth

1. The project as it is plays the truth: its daily discharge at `--channel` (the gauge).
2. A gauge record is made from that discharge with a known rating curve (b, h0, and a set so the
   median flow reads `--median-depth` m above h0), reading errors, rounding to the centimetre and
   missing days. The truth's mean annual ET stands for the satellite ET (with `--et-error`), and
   `--gaugings` discharge measurements are taken at spread-out stages (with 5 % error).
3. Each parameter is moved alone away from the truth (`--sweep`), the runs are scored against the
   synthetic record with a rating curve fitted to each run, and the scores are set beside how far
   each run's discharge really is from the truth's.

The question it answers: does the stage-based score point back to the truth, and which of its parts
see a wrong volume? Results go to `<out>/results.csv`, `<out>/truth.json` and `<out>/summary.md`.
"""

from __future__ import annotations

import argparse
import csv
import json
import os

import numpy as np

from . import metrics
from .objective import Observations, evaluate
from .runner import run_sets
from .stage import Rating, synthetic_stage

DEFAULT_SWEEP = "cn2:pctchg=-20,-10,10,20;alpha:pctchg=-50,-25,25,50;epco:absval=0.2,0.35,0.65,0.8;deep_seep:absval=0.001,0.1,0.2,0.3"


def parse_sweep(text):
    out = []
    for part in text.split(";"):
        key, vals = part.split("=")
        out.extend((key.strip(), float(v)) for v in vals.split(","))
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--exe", default="swatplus-rust")
    p.add_argument("--project", required=True)
    p.add_argument("--channel", type=int, required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--threads", type=int, default=0)
    p.add_argument("--b", type=float, default=1.8)
    p.add_argument("--h0", type=float, default=0.5)
    p.add_argument("--median-depth", type=float, default=1.5, help="stage above h0 at the median flow (m)")
    p.add_argument("--noise", type=float, default=0.02, help="reading error, standard deviation (m)")
    p.add_argument("--resolution", type=float, default=0.01)
    p.add_argument("--missing", type=float, default=0.05)
    p.add_argument("--et-error", type=float, default=0.0, help="relative error of the 'satellite' ET")
    p.add_argument("--gaugings", type=int, default=0)
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--sweep", default=DEFAULT_SWEEP)
    p.add_argument("--cal-parms")
    a = p.parse_args(argv)
    os.makedirs(a.out, exist_ok=True)

    truth = run_sets(a.exe, a.project, [{}], [a.channel], names=["truth"], threads=a.threads,
                     workdir=os.path.join(a.out, "truth_run"), cal_parms=a.cal_parms)["truth"]
    if isinstance(truth, str):
        raise SystemExit(f"the truth run failed: {truth}")
    q_true = truth.flo[a.channel]
    q_med = float(np.median(q_true[q_true > 0]))
    rating = Rating(q_med / a.median_depth**a.b, a.b, a.h0)
    rng = np.random.default_rng(a.seed + 1)
    stage = synthetic_stage(q_true, rating, a.noise, a.resolution, a.missing, a.seed)
    et_true = float(np.mean(truth.et) * 365.25)
    gaugings = []
    if a.gaugings > 0:
        for qq in np.quantile(q_true[q_true > 0], np.linspace(0.05, 0.95, a.gaugings)):
            gaugings.append((float(rating.stage(qq)), float(qq * (1 + rng.normal(0, 0.05)))))
    obs = Observations(stage, et_true * (1 + a.et_error), gaugings)
    with open(os.path.join(a.out, "truth.json"), "w") as f:
        json.dump({"channel": a.channel, "rating": rating.__dict__, "median_flow": q_med, "et_annual_mm": et_true,
                   "noise_m": a.noise, "missing": a.missing, "et_error": a.et_error, "gaugings": gaugings,
                   "days": int(len(stage)), "first": str(truth.dates[0]), "last": str(truth.dates[-1])}, f, indent=1)

    sweep = [("truth", 0.0)] + parse_sweep(a.sweep)
    sets = [{} if k == "truth" else {k: v} for k, v in sweep]
    names = [f"{i:02d}" for i in range(len(sets))]
    runs = run_sets(a.exe, a.project, sets, [a.channel], names=names, threads=a.threads,
                    workdir=os.path.join(a.out, "sweep_runs"), cal_parms=a.cal_parms)
    rows = []
    for (key, val), n in zip(sweep, names):
        r = runs[n]
        if isinstance(r, str):
            rows.append({"parameter": key, "change": val, "error": r})
            continue
        q = r.flo[a.channel]
        s = evaluate(q, r.et, obs)
        rows.append({
            "parameter": key, "change": val,
            "distance": s.distance, "kge_stage": s.kge_stage.kge, "spearman": s.spearman,
            "et_bias": s.et_bias, "gauging_error": s.gauging_error, "stage_rmse_m": s.stage_rmse,
            "fit_a": s.rating.a, "fit_b": s.rating.b, "fit_h0": s.rating.h0,
            "a_over_true_a": s.rating.a / rating.a,
            "true_flow_pbias": metrics.pbias(q, q_true), "true_flow_kge": metrics.kge(q, q_true).kge,
        })
    cols = ["parameter", "change", "distance", "kge_stage", "spearman", "et_bias", "gauging_error", "stage_rmse_m",
            "fit_a", "fit_b", "fit_h0", "a_over_true_a", "true_flow_pbias", "true_flow_kge", "error"]
    with open(os.path.join(a.out, "results.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols)
        w.writeheader()
        for row in rows:
            w.writerow({k: row.get(k, "") for k in cols})
    write_summary(os.path.join(a.out, "summary.md"), a, rating, et_true, rows)
    print(f"done: {os.path.join(a.out, 'summary.md')}")


def write_summary(path, a, rating, et_true, rows):
    def f(v, d=3):
        return "" if v is None or v == "" else f"{v:.{d}f}"

    lines = [
        f"# Synthetic stage test, channel {a.channel}",
        "",
        f"Truth: the project as it is. Rating curve a = {rating.a:.3f}, b = {rating.b}, h0 = {rating.h0} m; "
        f"reading error {a.noise} m, rounded to {a.resolution} m, {a.missing:.0%} of days missing. "
        f"Mean annual ET {et_true:.0f} mm (satellite ET error {a.et_error:+.0%}); {a.gaugings} gaugings.",
        "",
        "| parameter | change | distance | KGE stage | Spearman | ET bias | fitted a / true a | fitted b | fitted h0 | true flow PBIAS % | true flow KGE |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for r in rows:
        if "error" in r:
            lines.append(f"| {r['parameter']} | {r['change']:g} | failed: {r['error']} |")
            continue
        lines.append(
            f"| {r['parameter']} | {r['change']:g} | {f(r['distance'])} | {f(r['kge_stage'])} | {f(r['spearman'])} | "
            f"{f(r['et_bias'])} | {f(r['a_over_true_a'])} | {f(r['fit_b'], 2)} | {f(r['fit_h0'], 2)} | "
            f"{f(r['true_flow_pbias'], 1)} | {f(r['true_flow_kge'])} |")
    with open(path, "w") as fh:
        fh.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
