//! H1 (`docs/roadmap.md`): `channel_hyd_day.txt` is written only when asked for, changes no other
//! table, and reads the same flow the channel table prints.

use std::path::PathBuf;

use swat_engine::channel::{ChHyd, ChWat};
use swat_engine::hyd::Hyd;
use swat_engine::run::{Day, HruWeather, OutputSink, RunOptions, run_with};
use swat_io::{DirSource, load_project};
use swat_output::{MemStore, TextTables};

fn osu() -> swat_model::Project {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/Osu_1hru");
    load_project(&DirSource::new(dir)).unwrap().0
}

#[test]
fn the_table_changes_nothing_else() {
    let p = osu();
    for fixes in [false, true] {
        let tables = |on: bool| {
            let mut t = TextTables::new("hyd", MemStore::new(), &p.print);
            t.fixes = fixes;
            t.channel_hyd = on;
            run_with(&p, &mut t, RunOptions { fixes }).unwrap();
            t.finish().unwrap().files
        };
        let (off, mut on) = (tables(false), tables(true));
        assert!(!off.contains_key("channel_hyd_day.txt"));
        let hyd = on.remove("channel_hyd_day.txt").expect("channel_hyd_day.txt");
        assert_eq!(off, on, "the other tables changed (fixes {fixes})");
        let text = String::from_utf8(hyd).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[1].contains("flo_out") && lines[1].contains("dep_mann"));
        // One channel, every printed day (print.prt's window, after nyskip).
        let printed = p.time.days().iter().filter(|d| d.year >= p.time.yrc_start + p.print.nyskip).count();
        assert_eq!(lines.len() - 3, printed);
        for l in &lines[3..] {
            assert_eq!(l.split_whitespace().count(), 7 + 21, "{l}");
        }
    }
}

/// The channel's outflow and hydraulics as the engine hands them out.
#[derive(Default)]
struct Both(Vec<(f32, ChHyd)>, Option<f32>);

impl OutputSink for Both {
    fn hru_day(&mut self, _: Day, _: usize, _: i32, _: &str, _: &HruWeather, _: Option<&swat_engine::hru::HruDay>) {}
    fn cha_day(&mut self, _: Day, _: usize, _: i32, _: &str, _: &ChWat, _: &Hyd, _: &Hyd, hout: &Hyd, _: f32) {
        self.1 = Some(hout.flo);
    }
    fn cha_hyd(&mut self, _: Day, _: usize, _: i32, _: &str, h: &ChHyd) {
        self.0.push((self.1.take().expect("the channel's day comes first"), *h));
    }
}

#[test]
fn the_hydraulics_read_the_channel_s_flow() {
    let p = osu();
    for fixes in [false, true] {
        let mut b = Both::default();
        run_with(&p, &mut b, RunOptions { fixes }).unwrap();
        assert!(!b.0.is_empty());
        let mut wet = 0;
        for (flo, h) in &b.0 {
            assert!((h.out.flo_rate - flo).abs() <= 1.0e-5 * flo.max(1.0), "{} {flo}", h.out.flo_rate);
            let (d, v, w) = h.manning_out;
            if *flo > 0.0 {
                wet += 1;
                assert!(d > 0.0 && v > 0.0 && w > 0.0 && h.out.dep > 0.0);
            } else {
                assert_eq!((d, v, w), (0.0, 0.0, 0.0));
            }
            assert!(h.bankfull_dep > 0.0 && h.vol_ch >= 0.0 && h.vol_fp >= 0.0);
        }
        assert!(wet > 0);
    }
}
