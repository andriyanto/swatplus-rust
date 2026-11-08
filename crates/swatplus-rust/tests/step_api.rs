//! E1 (`docs/roadmap.md`): a simulation run a day at a time is the same run as `run_with`, and a
//! simulation copied mid-run goes on exactly as the original.

use std::path::PathBuf;

use swat_engine::hyd::Hyd;
use swat_engine::run::{Day, HruWeather, OutputSink, Period, RunOptions, Simulation, run_with};
use swat_io::{DirSource, load_project};
use swat_model::Project;
use swat_output::{MemStore, TextTables};

fn project(name: &str) -> Project {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata").join(name);
    load_project(&DirSource::new(dir)).unwrap().0
}

fn tables(p: &Project, fixes: bool) -> TextTables<MemStore> {
    let mut t = TextTables::new("step api", MemStore::new(), &p.print);
    t.fixes = fixes;
    t
}

/// Every table `run_with` writes.
fn whole(p: &Project, fixes: bool) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut t = tables(p, fixes);
    run_with(p, &mut t, RunOptions { fixes, ..Default::default() }).unwrap();
    t.finish().unwrap().files
}

#[test]
fn stepped_run_is_the_same_run() {
    for name in ["Osu_1hru", "Ames_sub1"] {
        let p = project(name);
        for fixes in [false, true] {
            let mut t = tables(&p, fixes);
            let mut sim = Simulation::new(&p, RunOptions { fixes, ..Default::default() }).unwrap();
            let (_, total) = sim.days();
            let mut n = 0;
            while sim.step(&mut t).unwrap() {
                n += 1;
                // Looking at the state between days changes nothing.
                assert_eq!(sim.days().0, n);
                let _ = (sim.date(), sim.hrus().map(<[_]>::len), sim.network().is_some(), sim.report());
            }
            assert_eq!(n, total, "{name}");
            assert!(sim.is_finished());
            assert!(!sim.step(&mut t).unwrap(), "a finished simulation stays finished");
            sim.finish(&mut t);
            let stepped = t.finish().unwrap().files;
            let all = whole(&p, fixes);
            assert_eq!(stepped.keys().collect::<Vec<_>>(), all.keys().collect::<Vec<_>>(), "{name}");
            for (k, v) in &all {
                assert!(stepped[k] == *v, "{name} {k} (fixes {fixes}) differs");
            }
        }
    }
}

/// Records what the engine hands out, as text.
#[derive(Default)]
struct Record(Vec<String>);

impl OutputSink for Record {
    fn hru_day(&mut self, day: Day, hru: usize, _gis_id: i32, _name: &str, w: &HruWeather, d: Option<&swat_engine::hru::HruDay>) {
        self.0.push(format!("hru {day:?} {hru} {w:?} {:?}", d.map(|d| format!("{:?}", d.wb))));
    }
    fn cha_day(
        &mut self,
        day: Day,
        ich: usize,
        _gis_id: i32,
        _name: &str,
        _w: &swat_engine::channel::ChWat,
        stor: &Hyd,
        hin: &Hyd,
        hout: &Hyd,
        wtemp: f32,
    ) {
        self.0.push(format!("cha {day:?} {ich} {stor:?} {hin:?} {hout:?} {wtemp}"));
    }
    fn aqu_day(&mut self, day: Day, iaq: usize, _gis_id: i32, _name: &str, d: &swat_engine::route::AquDay) {
        self.0.push(format!("aqu {day:?} {iaq} {d:?}"));
    }
    fn ru_day(&mut self, day: Day, name: &str, hd: &Hyd) {
        self.0.push(format!("ru {day:?} {name} {hd:?}"));
    }
    fn basin_crop_yld(&mut self, p: Period, yrc: i32, iplt: usize, plant: &str, area: f32, yld: f32, t_ha: f32) {
        self.0.push(format!("yld {p:?} {yrc} {iplt} {plant} {area} {yld} {t_ha}"));
    }
}

#[test]
fn a_copied_simulation_goes_on_as_the_original() {
    let p = project("Ames_sub1");
    for fixes in [false, true] {
        let mut whole = Record::default();
        let mut sim = Simulation::new(&p, RunOptions { fixes, ..Default::default() }).unwrap();
        // Past a year's end, into the second year.
        for _ in 0..400 {
            assert!(sim.step(&mut whole).unwrap());
        }
        let before = whole.0.len();
        let mut copy = sim.clone();
        while sim.step(&mut whole).unwrap() {}
        let mut branch = Record::default();
        while copy.step(&mut branch).unwrap() {}
        assert!(!branch.0.is_empty());
        assert!(whole.0[before..] == branch.0[..], "the copy went another way (fixes {fixes})");
    }
}

/// E2: a run started from a checkpoint is the run that went on from it, and a spin-up without
/// output then tables gives the tables of the rest of the run.
#[test]
fn a_checkpoint_starts_the_rest_of_the_run() {
    use swat_engine::run::NoOutput;
    let p = project("Ames_sub1");
    for fixes in [false, true] {
        let opts = RunOptions { fixes, ..Default::default() };
        let first = p.time.days()[0];
        let year = first.year + 1;
        let mut sim = Simulation::new(&p, opts).unwrap();
        assert!(sim.run_until(&mut NoOutput, year, 120).unwrap());
        assert_eq!((sim.date().yr, sim.date().jday), (year, 120));
        assert!(!sim.run_until(&mut NoOutput, year, 100).unwrap(), "a day already run");
        assert!(!sim.run_until(&mut NoOutput, 1800, 1).unwrap(), "a day outside the period");
        let start = sim.checkpoint();
        // Two runs from the checkpoint, and the original going on: the same days.
        let mut a = Record::default();
        let mut b = Record::default();
        let mut c = Record::default();
        let mut one = start.clone();
        let mut two = start.checkpoint();
        while one.step(&mut a).unwrap() {}
        while two.step(&mut b).unwrap() {}
        while sim.step(&mut c).unwrap() {}
        assert!(!a.0.is_empty() && a.0 == b.0 && a.0 == c.0, "fixes {fixes}");
        // The days after the checkpoint are the whole run's last days.
        let mut whole = Record::default();
        let mut w = Simulation::new(&p, opts).unwrap();
        while w.step(&mut whole).unwrap() {}
        assert!(whole.0.ends_with(&a.0), "fixes {fixes}");
    }
}

#[test]
fn the_date_is_the_last_day_run_at_a_year_s_end() {
    use swat_engine::run::NoOutput;
    let p = project("Osu_1hru");
    let mut sim = Simulation::new(&p, RunOptions::default()).unwrap();
    let y = p.time.days()[0].year;
    sim.run_until(&mut NoOutput, y, 365).unwrap();
    assert_eq!((sim.date().yr, sim.date().jday, sim.date().mon, sim.date().day), (y, 365, 12, 31));
}
