//! `swatplus-rust`: the command line program.
//!
//! ```text
//! swatplus-rust inspect <TxtInOut folder>
//! swatplus-rust run <TxtInOut folder> [--out <folder>]
//! swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]
//! ```
//!
//! `run` covers the daily weather and, for HRUs without management, the water balance, plants,
//! nutrients and losses, and writes the `hru_*_day.txt` tables with the columns it computes.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]

mod compare;
mod output;

use std::process::ExitCode;

use swat_io::{DirSource, load_project};
use swat_model::{ClimateVar, ObjectKind};

const USAGE: &str = "usage:
  swatplus-rust inspect <TxtInOut folder>
  swatplus-rust run <TxtInOut folder> [--out <folder>] [--fix-swat]
  swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["inspect", dir] => inspect(dir),
        ["run", dir, opts @ ..] => {
            let mut out = *dir;
            let mut fixes = false;
            let mut it = opts.iter();
            while let Some(o) = it.next() {
                match (*o, it.clone().next()) {
                    ("--out", Some(v)) => {
                        out = v;
                        it.next();
                    }
                    ("--fix-swat", _) => fixes = true,
                    _ => {
                        eprintln!("{USAGE}");
                        return ExitCode::from(2);
                    }
                }
            }
            run(dir, out, fixes)
        }
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

fn run(dir: &str, out: &str, fixes: bool) -> ExitCode {
    let (project, _) = match load_project(&DirSource::new(dir)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let title = format!(" {:<24} swatplus-rust {}", project.cio.title.trim_end_matches(':'), env!("CARGO_PKG_VERSION"));
    let out = std::path::Path::new(out);
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("error: {}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    let mut tables = output::TextTables::new(&title, out);
    tables.fixes = fixes;
    if fixes {
        eprintln!("note: --fix-swat: SWAT+'s bugs corrected, results will differ from SWAT+ (docs/swat-fixes.md)");
    }
    match swat_engine::run::run_with(&project, &mut tables, swat_engine::run::RunOptions { fixes }) {
        Ok(report) => {
            if let Some(why) = report.hru_skipped {
                eprintln!("note: HRU water balance not simulated ({why}); hru_wb_day.txt has precipitation only");
            }
            if let Some(why) = report.channel_skipped {
                eprintln!("note: channels not routed ({why}); they pass their inflow on");
            }
            if let Some(why) = report.routing_skipped {
                eprintln!("note: routing units and aquifers not simulated ({why})");
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    }
    if let Err(e) = tables.finish() {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    println!("wrote the daily HRU tables to {}", out.display());
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
    let open = |path: &str| -> Result<std::io::BufReader<std::fs::File>, String> {
        std::fs::File::open(path).map(std::io::BufReader::new).map_err(|e| format!("{path}: {e}"))
    };
    let (e, a) = match (open(expected), open(actual)) {
        (Ok(e), Ok(a)) => (e, a),
        (Err(err), _) | (_, Err(err)) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    let label = std::path::Path::new(actual).file_name().and_then(|n| n.to_str()).unwrap_or(actual);
    match compare::compare(e, a, abs, rel, label) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
