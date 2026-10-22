//! `swatplus-rust`: the command line program.
//!
//! For now it reads a scenario and reports what it holds; `run` arrives with the engine.
//!
//! ```text
//! swatplus-rust inspect <TxtInOut folder>
//! ```
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]

use std::process::ExitCode;

use swat_io::{DirSource, load_project};
use swat_model::{ClimateVar, ObjectKind};

const USAGE: &str = "usage: swatplus-rust inspect <TxtInOut folder>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["inspect", dir] => inspect(dir),
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
