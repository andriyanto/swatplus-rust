//! M1 (`docs/roadmap.md`): a calibration built in memory is applied exactly as `calibration.cal`
//! is, so a calibration program can run one project many times with different parameters without
//! writing a file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use swat_engine::run::{RunOptions, Simulation, run_with, run_with_calibration};
use swat_io::{DirSource, load_project};
use swat_model::Project;
use swat_model::cal::{CalUpdate, Calibration};
use swat_output::{MemStore, TextTables};

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata").join(name)
}

fn load(dir: &Path) -> Project {
    load_project(&DirSource::new(dir.to_path_buf())).unwrap().0
}

/// The tables of a run, with the project's calibration or with `cal`.
fn tables(p: &Project, cal: Option<&Calibration>) -> BTreeMap<String, Vec<u8>> {
    let mut t = TextTables::new("calibration", MemStore::new(), &p.print);
    match cal {
        Some(c) => run_with_calibration(p, &mut t, RunOptions::default(), c).unwrap(),
        None => run_with(p, &mut t, RunOptions::default()).unwrap(),
    };
    t.finish().unwrap().files
}

fn cal_parms() -> Vec<swat_model::cal::CalParm> {
    let path = testdata("Osu_1hru").join("cal_parms.cal");
    swat_io::cal::read_cal_parms("cal_parms.cal", &std::fs::read_to_string(path).unwrap()).unwrap()
}

fn changes() -> Vec<CalUpdate> {
    vec![CalUpdate::all("cn2", "pctchg", -10.0), CalUpdate::all("esco", "absval", 0.8), CalUpdate::all("alpha", "pctchg", 20.0)]
}

/// A copy of Ames_sub1 with these changes written to its `calibration.cal`, as a user of SWAT+
/// would make them.
fn written_copy() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("swatplus-rust-m1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(testdata("Ames_sub1")).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
    }
    std::fs::copy(testdata("Osu_1hru").join("cal_parms.cal"), dir.join("cal_parms.cal")).unwrap();
    let rows: String = changes()
        .iter()
        .map(|u| format!("{:<12}{:<12}{:>12}           0     0     0     0     0     0     0     0\n", u.name, u.chg_typ, u.val))
        .collect();
    std::fs::write(
        dir.join("calibration.cal"),
        format!(
            "calibration.cal: written by the M1 test\n{}\nNAME CHG_TYP VAL CONDS LYR1 LYR2 YEAR1 YEAR2 DAY1 DAY2 OBJ_TOT\n{rows}",
            changes().len()
        ),
    )
    .unwrap();
    let cio = std::fs::read_to_string(dir.join("file.cio")).unwrap();
    let cio: String = cio
        .lines()
        // All of the line's fields: file.cio is read list-directed, and a short line would take
        // the next line's.
        .map(|l| {
            if l.starts_with("chg ") {
                format!("chg               cal_parms.cal     calibration.cal{}", "   null".repeat(7))
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("file.cio"), cio + "\n").unwrap();
    dir
}

#[test]
fn a_calibration_in_memory_is_the_calibration_written_to_its_files() {
    let ames = load(&testdata("Ames_sub1"));
    let cal = Calibration { parms: cal_parms(), updates: changes() };
    let in_memory = tables(&ames, Some(&cal));

    let dir = written_copy();
    let written = load(&dir);
    assert_eq!(written.cal_upd, changes(), "the copy's calibration.cal reads back as the changes");
    let from_files = tables(&written, None);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(in_memory.keys().collect::<Vec<_>>(), from_files.keys().collect::<Vec<_>>());
    for (k, v) in &from_files {
        assert!(in_memory[k] == *v, "{k} differs");
    }
    // And the changes did change the run.
    let plain = tables(&ames, None);
    assert!(plain["hru_wb_day.txt"] != in_memory["hru_wb_day.txt"]);
}

#[test]
fn the_project_s_own_calibration_gives_the_usual_run() {
    let osu = load(&testdata("Osu_1hru"));
    assert!(!osu.cal_upd.is_empty());
    assert_eq!(tables(&osu, Some(&Calibration::of(&osu))), tables(&osu, None));
}

#[test]
fn one_project_runs_with_many_calibrations() {
    // A calibration program loads the project once and runs it again and again: no run leaves
    // anything behind for the next.
    let ames = load(&testdata("Ames_sub1"));
    let a = Calibration { parms: cal_parms(), updates: vec![CalUpdate::all("cn2", "pctchg", 15.0)] };
    let b = Calibration { parms: cal_parms(), updates: vec![CalUpdate::all("cn2", "pctchg", -15.0)] };
    let first = tables(&ames, Some(&a));
    let other = tables(&ames, Some(&b));
    assert!(first["hru_wb_day.txt"] != other["hru_wb_day.txt"]);
    assert_eq!(tables(&ames, Some(&a)), first);
}

#[test]
fn a_parameter_cal_parms_doesn_t_list_is_refused() {
    let ames = load(&testdata("Ames_sub1"));
    let cal = Calibration { parms: cal_parms(), updates: vec![CalUpdate::all("no_such_parm", "pctchg", 5.0)] };
    let err = Simulation::with_calibration(&ames, RunOptions::default(), &cal).err().unwrap();
    assert!(err.contains("no_such_parm"), "{err}");
    // Without any cal_parms.cal every change is unknown.
    let bare = Calibration { parms: Vec::new(), updates: changes() };
    assert!(Simulation::with_calibration(&ames, RunOptions::default(), &bare).is_err());
}

/// The tables of a stepped run of `sim` to its end.
fn run_out(p: &Project, mut sim: Simulation, mut t: TextTables<MemStore>) -> BTreeMap<String, Vec<u8>> {
    let _ = p;
    while sim.step(&mut t).unwrap() {}
    sim.finish(&mut t);
    t.finish().unwrap().files
}

#[test]
fn changes_applied_before_the_first_day_are_the_changes_set_up_with_the_run() {
    // `Simulation::calibrate` before any day has run is the run set up with those changes: the
    // parameters change in the same place, and nothing else does.
    let ames = load(&testdata("Ames_sub1"));
    let none = Calibration { parms: cal_parms(), updates: Vec::new() };
    let cal = Calibration { parms: cal_parms(), updates: changes() };
    let mut sim = Simulation::with_calibration(&ames, RunOptions::default(), &none).unwrap();
    sim.calibrate(&cal).unwrap();
    let late = run_out(&ames, sim, TextTables::new("calibration", MemStore::new(), &ames.print));
    assert_eq!(late, tables(&ames, Some(&cal)));
}

#[test]
fn changes_applied_during_a_run_change_only_what_follows() {
    // A shared spin-up: the days before the change are the plain run's, the days after differ.
    let ames = load(&testdata("Ames_sub1"));
    let none = Calibration { parms: cal_parms(), updates: Vec::new() };
    let cal = Calibration { parms: cal_parms(), updates: changes() };
    let mut sim = Simulation::with_calibration(&ames, RunOptions::default(), &none).unwrap();
    let mut t = TextTables::new("calibration", MemStore::new(), &ames.print);
    // The change goes in after day 200 of the first printed year.
    let year = ames.time.yrc_start + ames.print.nyskip;
    assert!(sim.run_until(&mut t, year, 200).unwrap());
    sim.calibrate(&cal).unwrap();
    let late = run_out(&ames, sim, t);
    let plain = tables(&ames, Some(&none));
    let rows = |f: &BTreeMap<String, Vec<u8>>, after: bool| -> Vec<String> {
        String::from_utf8(f["hru_wb_day.txt"].clone())
            .unwrap()
            .lines()
            .filter(|l| {
                let w: Vec<i32> = l.split_whitespace().take(4).map_while(|v| v.parse().ok()).collect();
                w.len() == 4 && ((w[3], w[0]) > (year, 200)) == after
            })
            .map(String::from)
            .collect()
    };
    assert!(!rows(&plain, false).is_empty());
    assert_eq!(rows(&late, false), rows(&plain, false), "the days up to the change");
    assert_ne!(rows(&late, true), rows(&plain, true), "the days after it");
}

#[test]
fn a_change_that_would_set_the_state_up_again_is_refused_during_a_run() {
    let ames = load(&testdata("Ames_sub1"));
    let none = Calibration { parms: cal_parms(), updates: Vec::new() };
    let mut sim = Simulation::with_calibration(&ames, RunOptions::default(), &none).unwrap();
    let awc = Calibration { parms: cal_parms(), updates: vec![CalUpdate::all("awc", "pctchg", 10.0)] };
    assert!(sim.calibrate(&awc).unwrap_err().contains("sets the state up again"));
}
