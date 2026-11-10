# Calibration in memory

M1 in [`roadmap.md`](roadmap.md). A calibration program runs one project hundreds or thousands of
times, each time with other parameter values. SWAT+ takes them from `calibration.cal` (hard
calibration, `cal_parms.cal` holding each parameter's kind and range), so a program built on
SWAT+ writes that file, or edits the input files, before every run. swatplus-rust takes the same
changes in memory: the project is read once, and each run gets its own list of changes.

The changes are applied exactly as `calibration.cal`'s are (`proc_cal` → `cal_conditions`, after
the objects are set up; the parameters and conditions are in [`engine1.md`](engine1.md)). A run
with a calibration in memory is therefore the run of a copy of the project with that calibration
written to its files, which is the run SWAT+ makes of it.

## In code

```rust
use swat_model::cal::{Calibration, CalUpdate};

let project = swat_io::load_project(&swat_io::DirSource::new(dir))?.0;
let cal = Calibration::with_updates(&project, vec![
    CalUpdate::all("cn2", "pctchg", -10.0),   // every HRU's CN2 10 % lower
    CalUpdate::all("esco", "absval", 0.8),     // ESCO set to 0.8
    CalUpdate::all("alpha", "pctchg", 20.0),   // aquifers' baseflow alpha 20 % higher
]);
let report = swat_engine::run::run_with_calibration(&project, &mut tables, opts, &cal)?;
```

- `Calibration::of(project)` is the project's own calibration; `Calibration::with_updates` keeps
  its `cal_parms.cal` and replaces its changes. `parms` can also be read from another
  `cal_parms.cal` (`swat_io::cal::read_cal_parms`).
- `CalUpdate::all(name, chg_typ, val)` changes the parameter for every object of its kind:
  `absval` sets it, `abschg` adds to it, `pctchg` changes it by a percentage, `relchg` multiplies
  it by 1 + `val`. Each result is held to the parameter's range in `cal_parms.cal`. Conditions,
  soil layers and object lists are the other fields of `CalUpdate`, as in `calibration.cal`.
- `Simulation::with_calibration` is the stepped form (`docs/runner.md`): with `run_until` and
  `checkpoint`, a spin-up can be run once and each parameter set started from a copy of it (M2).
- A change of a parameter that `cal_parms.cal` doesn't list is refused, and so is a calibration
  under which the HRUs can't be simulated: a project run with its own files would then run
  without its HRUs and only report it, which a calibration program must not mistake for a result.
- The project is only borrowed: one loaded project serves every run, on any number of threads.

## On the command line

```text
swatplus-rust run <TxtInOut> --set cn2 pctchg -10 --set esco absval 0.8
swatplus-rust run <TxtInOut> --calibration trial-17.cal
swatplus-rust run <TxtInOut> --cal-parms cal_parms.cal --set cn2 pctchg -10
```

- `--set <name> <absval|abschg|pctchg|relchg> <value>`, as many as needed, changes a parameter
  for every object of its kind, after the project's own changes.
- `--calibration <file>` replaces the project's `calibration.cal` with another file in the same
  format; `--cal-parms <file>` replaces its `cal_parms.cal`. A project without a
  `cal_parms.cal` (Ames_sub1) needs `--cal-parms`.
- The project's files are not touched.

## Checked

- `crates/swatplus-rust/tests/calibration.rs`: Ames_sub1 with three changes in memory gives the
  same tables, byte for byte, as a copy of Ames_sub1 with the same changes written to its
  `calibration.cal` and `file.cio`; Osu_1hru with its own calibration passed in memory is its
  usual run; one loaded project run with several calibrations in turn gives each its own run; an
  unknown parameter is refused.
- The copy of Ames_sub1 with those changes, run by the SWAT+ Fortran reference (commit 6f51536,
  gfortran), matches the in-memory run in every column of `hru_wb`, `hru_nb`, `hru_pw` (daily
  and average annual) and `hru_ls` (average annual), and in the crop yields.
- Every table of the 37 validation scenarios, faithful and `--fix-swat`, is unchanged.
