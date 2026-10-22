//! `swatplus-rust`: the command line program.
//!
//! ```text
//! swatplus-rust inspect <TxtInOut folder>
//! swatplus-rust run <TxtInOut folder> [--out <folder>]
//! swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]
//! ```
//!
//! `run` covers the daily weather so far and writes `hru_wb_day.txt` (precipitation) and
//! `hru_pw_day.txt` (weather) with those columns only.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]

mod compare;
mod output;

use std::process::ExitCode;

use swat_io::{DirSource, load_project};
use swat_model::{ClimateVar, ObjectKind};

const USAGE: &str = "usage:
  swatplus-rust inspect <TxtInOut folder>
  swatplus-rust run <TxtInOut folder> [--out <folder>]
  swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["inspect", dir] => inspect(dir),
        ["run", dir] => run(dir, dir),
        ["run", dir, "--out", out] => run(dir, out),
        ["compare", expected, actual, opts @ ..] => compare_cmd(expected, actual, opts),
        ["--version"] | ["-V"] => {
            println!("swatplus-rust {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn inspect(dir: &str) -> ExitCode {
    let (project, report) = match load_project(&DirSource::new(dir)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let t = project.time;
    let days = t.days();
    println!("Scenario      {dir}");
    println!(
        "Period        {}-{:03} to {}-{:03}, {} years, {} days, step {}",
        t.yrc_start,
        t.normalized().day_start,
        t.yrc_end,
        days.last().map(|d| d.jday).unwrap_or(0),
        t.years(),
        days.len(),
        t.normalized().step
    );
    println!("Warm-up       {} years not printed", project.print.nyskip);
    println!("Objects");
    for kind in ObjectKind::ALL {
        let n = project.objects_of(kind).count();
        if n > 0 {
            let area: f32 = project.objects_of(kind).map(|o| o.area_ha).sum();
            println!("  {:<20} {n:>6}   {area:>12.2} ha", format!("{kind:?}"));
        }
    }
    println!("Weather       {} stations, {} generator stations", project.stations.len(), project.wgn.len());
    for var in ClimateVar::ALL {
        for s in project.series.iter().filter(|s| s.var == var) {
            let first = s.records.first().map(|r| format!("{}-{:03}", r.year, r.jday)).unwrap_or_default();
            let last = s.records.last().map(|r| format!("{}-{:03}", r.year, r.jday)).unwrap_or_default();
            println!("  {:<4} {:<24} {:>7} records  {first} to {last}", format!("{var:?}"), s.file, s.records.len());
        }
    }
    for f in &report.missing {
        println!("Missing       {f}");
    }
    for f in &report.unread {
        println!("Not read      {f} (ends in its header, as SWAT+ reads it)");
    }
    ExitCode::SUCCESS
}

fn run(dir: &str, out: &str) -> ExitCode {
    let (project, _) = match load_project(&DirSource::new(dir)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let title = format!(" {:<24} swatplus-rust {}", project.cio.title.trim_end_matches(':'), env!("CARGO_PKG_VERSION"));
    let mut tables = output::TextTables::new(&title);
    if let Err(e) = swat_engine::run::run(&project, &mut tables) {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    let out = std::path::Path::new(out);
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("error: {}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    for (name, text) in [("hru_wb_day.txt", &tables.wb), ("hru_pw_day.txt", &tables.pw)] {
        let path = out.join(name);
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("error: {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    println!("wrote hru_wb_day.txt and hru_pw_day.txt to {}", out.display());
    ExitCode::SUCCESS
}

fn compare_cmd(expected: &str, actual: &str, opts: &[&str]) -> ExitCode {
    let mut abs = 0.001_f64;
    let mut rel = 1.0e-4_f64;
    let mut it = opts.iter();
    while let Some(opt) = it.next() {
        let value = it.next().and_then(|v| v.parse::<f64>().ok());
        match (*opt, value) {
            ("--abs", Some(v)) => abs = v,
            ("--rel", Some(v)) => rel = v,
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let read = |path: &str| -> Result<compare::Table, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        compare::read_table(&text).map_err(|e| format!("{path}: {e}"))
    };
    let (e, a) = match (read(expected), read(actual)) {
        (Ok(e), Ok(a)) => (e, a),
        (Err(err), _) | (_, Err(err)) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    let label = std::path::Path::new(actual).file_name().and_then(|n| n.to_str()).unwrap_or(actual);
    if compare::compare(&e, &a, abs, rel, label) { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
