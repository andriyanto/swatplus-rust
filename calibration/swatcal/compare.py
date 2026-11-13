"""The optimisers compared on a synthetic world (M4 in docs/roadmap.md).

    python -m swatcal.compare --exe swatplus-rust --project <TxtInOut> --channel 1 --out cmp --years 2015-2019

A known truth: the project run with chosen values of the calibrated parameters (`--truth`). From its
discharge at the gauge, a stage record (known rating curve, 2 cm reading errors, rounded, 5 % of days
missing); from its ET, a "satellite" mean annual ET; from its flow, six discharge measurements (5 %
error). Each optimiser then gets the same number of runs (`--budget`) to find the truth back, under
three states of knowledge:

- `stage`: the stage record only;
- `stage+et`: and the satellite ET;
- `stage+et+gaugings`: and the six measurements.

For each, `results.csv` gives the best score found, how far its parameters are from the truth (in
the unit cube), and how good its discharge really is against the truth (KGE, PBIAS), which the
optimiser never sees. `convergence.csv` has the best score after each run. Runs are cached by
their parameters, so a point two searches share is run once.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import shutil

import numpy as np

from . import metrics
from .objective import Observations, evaluate
from .optimize import bayes_opt, random_search, sufi2
from .params import Param, Space
from .runner import run_sets
from .stage import Rating, synthetic_stage

SPACE = Space((
    Param("cn2:pctchg", -25.0, 25.0),
    Param("alpha:pctchg", -60.0, 60.0),
    Param("epco:absval", 0.1, 1.0),
    Param("deep_seep:absval", 0.001, 0.4),
))
TRUTH = (8.0, -20.0, 0.6, 0.15)
SCENARIOS = ("stage", "stage+et", "stage+et+gaugings")


def shortened_copy(project: str, years: str, dest: str) -> str:
    """A copy of the project simulating only `years` (first-last), print.prt unchanged."""
    y0, y1 = (int(v) for v in years.split("-"))
    if os.path.exists(dest):
        shutil.rmtree(dest)
    shutil.copytree(project, dest)
    path = os.path.join(dest, "time.sim")
    lines = open(path).read().splitlines()
    f = lines[2].split()
    f[1], f[3] = str(y0), str(y1)
    lines[2] = "".join(f"{v:>10}" for v in f)
    open(path, "w").write("\n".join(lines) + "\n")
    return dest


class World:
    """The truth, the synthetic observations, and a cache of runs by parameter set."""

    def __init__(self, a):
        self.a = a
        self.cache: dict[tuple, object] = {}
        self.work = os.path.join(a.out, "runs")
        self.calls = 0
        truth = self.run(np.array([a.truth]))[0]
        if isinstance(truth, str):
            raise SystemExit(f"the truth run failed: {truth}")
        self.q_true = truth.flo[a.channel]
        q_med = float(np.median(self.q_true[self.q_true > 0]))
        self.rating = Rating(q_med / 1.5**1.8, 1.8, 0.5)
        stage = synthetic_stage(self.q_true, self.rating, 0.02, 0.01, 0.05, a.seed)
        self.et_true = float(np.mean(truth.et) * 365.25)
        rng = np.random.default_rng(a.seed + 1)
        g = [(float(self.rating.stage(q)), float(q * (1 + rng.normal(0, 0.05))))
             for q in np.quantile(self.q_true[self.q_true > 0], np.linspace(0.05, 0.95, 6))]
        self.obs = {
            "stage": Observations(stage),
            "stage+et": Observations(stage, self.et_true),
            "stage+et+gaugings": Observations(stage, self.et_true, g),
        }

    def run(self, xs):
        keys = [tuple(np.round(x, 9)) for x in xs]
        todo = [k for k in dict.fromkeys(keys) if k not in self.cache]
        if todo:
            self.calls += 1
            names = [f"c{self.calls:03d}_{i:03d}" for i in range(len(todo))]
            res = run_sets(self.a.exe, self.a.project, SPACE.sets(np.array(todo)), [self.a.channel], names=names,
                           threads=self.a.threads, workdir=os.path.join(self.work, f"call{self.calls:03d}"))
            for k, n in zip(todo, names):
                self.cache[k] = res[n]
            shutil.rmtree(os.path.join(self.work, f"call{self.calls:03d}"), ignore_errors=True)
        return [self.cache[k] for k in keys]

    def objective(self, scenario):
        obs = self.obs[scenario]

        def f(xs):
            out = []
            for r in self.run(xs):
                out.append(np.inf if isinstance(r, str) else evaluate(r.flo[self.a.channel], r.et, obs).distance)
            return np.array(out)

        return f


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--exe", default="swatplus-rust")
    p.add_argument("--project", required=True)
    p.add_argument("--channel", type=int, required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--years", help="simulate only these years, e.g. 2015-2019 (a copy of the project)")
    p.add_argument("--budget", type=int, default=80)
    p.add_argument("--batch", type=int, default=2, help="runs per batch of Bayesian optimisation")
    p.add_argument("--threads", type=int, default=0)
    p.add_argument("--seed", type=int, default=1, help="seed of the synthetic record")
    p.add_argument("--seeds", default="1", help="seeds of the searches, e.g. 1,2,3: each method runs once per seed")
    p.add_argument("--truth", default=",".join(map(str, TRUTH)))
    p.add_argument("--methods", default="random,sufi2,bayes", help="random, sufi2, bayes (with trust region), bayes-global")
    p.add_argument("--scenarios", default=",".join(SCENARIOS))
    a = p.parse_args(argv)
    a.truth = [float(v) for v in a.truth.split(",")]
    os.makedirs(a.out, exist_ok=True)
    if a.years:
        a.project = shortened_copy(a.project, a.years, os.path.join(a.out, "project"))
    world = World(a)
    truth_u = SPACE.to_unit(a.truth)
    rows, curves = [], {}
    for seed in (int(v) for v in a.seeds.split(",")):
        for scenario in a.scenarios.split(","):
            f = world.objective(scenario)
            for method in a.methods.split(","):
                rng = np.random.default_rng(1000 + seed)
                if method == "random":
                    h = random_search(f, SPACE, a.budget, rng)
                elif method == "sufi2":
                    h = sufi2(f, SPACE, a.budget, rng)
                elif method == "bayes-global":
                    h = bayes_opt(f, SPACE, a.budget, rng, batch=a.batch, trust_region=False)
                else:
                    h = bayes_opt(f, SPACE, a.budget, rng, batch=a.batch)
                x, y = h.best()
                r = world.run(np.array([x]))[0]
                q = r.flo[a.channel]
                rows.append({
                    "seed": seed, "scenario": scenario, "method": method, "runs": h.n, "best_score": y,
                    "param_error": float(np.linalg.norm(SPACE.to_unit(x) - truth_u) / np.sqrt(SPACE.dim)),
                    **{k.key: float(v) for k, v in zip(SPACE.params, x)},
                    "true_flow_kge": metrics.kge(q, world.q_true).kge, "true_flow_pbias": metrics.pbias(q, world.q_true),
                })
                curves[f"{seed}/{scenario}/{method}"] = h.best_so_far().tolist()
                with open(os.path.join(a.out, "results.csv"), "w", newline="") as fh:
                    w = csv.DictWriter(fh, fieldnames=list(rows[0]))
                    w.writeheader()
                    w.writerows(rows)
                print(f"seed {seed} {scenario:<18} {method:<12} best {y:.4f}  flow KGE {rows[-1]['true_flow_kge']:.3f}  "
                      f"PBIAS {rows[-1]['true_flow_pbias']:+.1f} %  ({len(world.cache)} distinct runs so far)", flush=True)
    with open(os.path.join(a.out, "results.csv"), "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)
    with open(os.path.join(a.out, "convergence.json"), "w") as fh:
        json.dump(curves, fh)
    with open(os.path.join(a.out, "setup.json"), "w") as fh:
        json.dump({"space": [p.__dict__ for p in SPACE.params], "truth": a.truth, "budget": a.budget, "seed": a.seed,
                   "rating": world.rating.__dict__, "et_annual_mm": world.et_true, "years": a.years,
                   "distinct_runs": len(world.cache)}, fh, indent=1)
    plot(a.out, curves)


def plot(out, curves):
    # One line per method and scenario: the median over seeds of the best score so far.
    grouped = {}
    for k, c in curves.items():
        _, scen, meth = k.split("/")
        grouped.setdefault(f"{scen}/{meth}", []).append(c)
    curves = {k: np.median(np.array(v), axis=0).tolist() for k, v in grouped.items()}
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        return
    scen = list(dict.fromkeys(k.split("/")[0] for k in curves))
    fig, axes = plt.subplots(1, len(scen), figsize=(4.2 * len(scen), 3.4), sharey=True)
    for ax, s in zip(np.atleast_1d(axes), scen):
        for k, c in curves.items():
            if k.startswith(s + "/"):
                ax.plot(np.arange(1, len(c) + 1), c, label=k.split("/")[1])
        ax.set_title(s)
        ax.set_xlabel("model runs")
        ax.set_yscale("log")
    np.atleast_1d(axes)[0].set_ylabel("best score so far (distance)")
    np.atleast_1d(axes)[0].legend()
    fig.tight_layout()
    fig.savefig(os.path.join(out, "convergence.png"), dpi=130)


if __name__ == "__main__":
    main()
