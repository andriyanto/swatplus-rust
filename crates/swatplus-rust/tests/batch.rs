//! M2 (`docs/roadmap.md`): a batch of runs keeps the series a calibration needs, the values the
//! tables print, and gives each run the same result whatever runs beside it.

use std::path::PathBuf;

use swat_engine::batch::{run_batch, run_one};
use swat_engine::run::{RunOptions, run_with_calibration};
use swat_io::{DirSource, load_project};
use swat_model::Project;
use swat_model::cal::{CalUpdate, Calibration};
use swat_output::{MemStore, TextTables};

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata").join(name)
}

/// Osu_1hru with its channel and basin water balance printed daily.
fn osu_daily() -> Project {
    let mut p = load_project(&DirSource::new(testdata("Osu_1hru"))).unwrap().0;
    for o in p.print.objects.iter_mut() {
        if o.name == "channel_sd" || o.name == "basin_wb" {
            o.daily = "y".into();
        }
    }
    p
}

/// The project's calibration, then soil evaporation (`esco`) set to `esco`.
fn cal(p: &Project, esco: f32) -> Calibration {
    let mut c = Calibration::of(p);
    c.updates.push(CalUpdate::all("esco", "absval", esco));
    c
}

/// A column of a printed table, by its header.
fn column(text: &str, name: &str) -> Vec<f64> {
    let mut lines = text.lines().skip(1);
    let head: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    let k = head.iter().position(|h| *h == name).unwrap_or_else(|| panic!("no column {name}"));
    lines.skip(1).filter_map(|l| l.split_whitespace().nth(k)?.parse().ok()).collect()
}

fn close(series: &[f32], printed: &[f64], what: &str) {
    assert_eq!(series.len(), printed.len(), "{what}: days");
    for (i, (a, b)) in series.iter().zip(printed).enumerate() {
        let a = f64::from(*a);
        assert!((a - b).abs() <= 1e-3 + 1e-4 * b.abs(), "{what} day {}: {a} against {b}", i + 1);
    }
}

#[test]
fn the_series_are_what_the_tables_print() {
    let p = osu_daily();
    let c = cal(&p, 0.8);
    let mut t = TextTables::new("batch", MemStore::new(), &p.print);
    t.channel_hyd = true;
    run_with_calibration(&p, &mut t, RunOptions::default(), &c).unwrap();
    let files = t.finish().unwrap().files;
    let text = |n: &str| String::from_utf8(files[n].clone()).unwrap();

    let (s, _) = run_one(&p, RunOptions::default(), &c, &[1], true).unwrap();
    assert!(!s.days.is_empty());
    assert_eq!(s.days.len(), s.precip.len());
    close(&s.flo[0], &column(&text("channel_sd_day.txt"), "flo_out"), "flo_out");
    close(&s.dep_out[0], &column(&text("channel_hyd_day.txt"), "dep_out"), "dep_out");
    close(&s.dep_mann[0], &column(&text("channel_hyd_day.txt"), "dep_mann"), "dep_mann");
    close(&s.precip, &column(&text("basin_wb_day.txt"), "precip"), "precip");
    close(&s.et, &column(&text("basin_wb_day.txt"), "et"), "et");
}

#[test]
fn a_run_gives_the_same_series_whatever_runs_beside_it() {
    let p = osu_daily();
    let cals: Vec<Calibration> = [0.5, 0.65, 0.8, 0.9, 1.0].iter().map(|&v| cal(&p, v)).collect();
    let alone: Vec<_> = cals.iter().map(|c| run_one(&p, RunOptions::default(), c, &[1], false).unwrap().0).collect();
    for threads in [1, 3] {
        let batch = run_batch(&p, RunOptions::default(), &cals, &[1], false, threads, &|_| {});
        let got: Vec<_> = batch.into_iter().map(|r| r.unwrap().0).collect();
        assert_eq!(got, alone, "{threads} at a time");
    }
    // The parameter sets do change the flow.
    assert!(alone[0].flo != alone[4].flo);
}

#[test]
fn a_failed_run_is_reported_in_its_place() {
    let p = osu_daily();
    let mut bad = cal(&p, 0.7);
    bad.updates.push(CalUpdate::all("no_such_parm", "pctchg", 1.0));
    let cals = vec![cal(&p, 0.6), bad, cal(&p, 0.9)];
    let r = run_batch(&p, RunOptions::default(), &cals, &[1], false, 2, &|_| {});
    assert!(r[0].is_ok() && r[2].is_ok());
    assert!(r[1].as_ref().err().is_some_and(|e| e.contains("no_such_parm")));
}
