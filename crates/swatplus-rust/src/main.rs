//! `swatplus-rust`: the command line program.
//!
//! ```text
//! swatplus-rust inspect <TxtInOut folder>
//! swatplus-rust run <TxtInOut folder> [--out <folder>] [--fix-swat] [--format text|parquet|both] [--progress json]
//! swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]
//! ```
//!
//! `run` covers the daily weather and, for HRUs without management, the water balance, plants,
//! nutrients and losses, and writes the `hru_*_day.txt` tables with the columns it computes.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]

mod compare;

use std::process::ExitCode;

use swat_io::{DirSource, load_project};
use swat_model::{ClimateVar, ObjectKind};

const USAGE: &str = "usage:
  swatplus-rust inspect <TxtInOut folder>
  swatplus-rust run <TxtInOut folder> [--out <folder>] [--fix-swat] [--format text|parquet|both] [--progress json]
  swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["inspect", dir] => inspect(dir),
        ["run", dir, opts @ ..] => {
            let mut o = RunArgs { out: dir, fixes: false, text: true, parquet: false, json: false };
            let mut it = opts.iter();
            while let Some(a) = it.next() {
                match (*a, it.clone().next().copied()) {
                    ("--out", Some(v)) => {
                        o.out = v;
                        it.next();
                    }
                    ("--fix-swat", _) => o.fixes = true,
                    ("--format", Some(v)) if matches!(v, "text" | "parquet" | "both") => {
                        o.text = v != "parquet";
                        o.parquet = v != "text";
                        it.next();
                    }
                    ("--progress", Some("json")) => {
                        o.json = true;
                        it.next();
                    }
                    _ => {
                        eprintln!("{USAGE}");
                        return ExitCode::from(2);
                    }
                }
            }
            run(dir, &o)
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

/// A line on standard output, ignoring a closed pipe.
fn emit(line: &str) {
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

struct RunArgs<'a> {
    out: &'a str,
    fixes: bool,
    /// Keep the text tables, write Parquet tables.
    text: bool,
    parquet: bool,
    /// Progress and events as JSON lines on standard output (`swat_output::progress`).
    json: bool,
}

fn run(dir: &str, o: &RunArgs) -> ExitCode {
    let started = std::time::Instant::now();
    let json = o.json;
    let note = |text: &str| {
        if json {
            emit(&swat_output::progress::text("note", text));
        } else {
            eprintln!("note: {text}");
        }
    };
    let fail = |text: &str| {
        if json {
            emit(&swat_output::progress::text("error", text));
        } else {
            eprintln!("error: {text}");
        }
        ExitCode::FAILURE
    };
    let (project, _) = match load_project(&DirSource::new(dir)) {
        Ok(v) => v,
        Err(e) => return fail(&e.to_string()),
    };
    let title = format!(" {:<24} swatplus-rust {}", project.cio.title.trim_end_matches(':'), env!("CARGO_PKG_VERSION"));
    let out = std::path::Path::new(o.out);
    if let Err(e) = std::fs::create_dir_all(out) {
        return fail(&format!("{}: {e}", out.display()));
    }
    let days = project.time.days();
    if json {
        let d = |k: usize| {
            days.get(k).map(|d| {
                let nd = if swat_model::time::is_leap(d.year) { swat_model::time::NDAYS_LEAP } else { swat_model::time::NDAYS_NOLEAP };
                let (mo, dm) = swat_engine::clock::xmon(&nd, d.jday);
                format!("{:04}-{mo:02}-{dm:02}", d.year)
            })
        };
        emit(&format!(
            "{{\"event\":\"start\",\"version\":\"{}\",\"days\":{},\"first\":\"{}\",\"last\":\"{}\"}}",
            env!("CARGO_PKG_VERSION"),
            days.len(),
            d(0).unwrap_or_default(),
            d(days.len().saturating_sub(1)).unwrap_or_default()
        ));
    }
    let mut tables = swat_output::TextTables::new(&title, swat_output::DirStore::new(out), &project.print);
    tables.fixes = o.fixes;
    if json {
        tables.progress = Some(swat_output::progress::Reporter::new(Box::new(emit)));
    }
    if o.fixes {
        note("--fix-swat: SWAT+'s bugs corrected, results will differ from SWAT+ (docs/swat-fixes.md)");
    }
    match swat_engine::run::run_with(&project, &mut tables, swat_engine::run::RunOptions { fixes: o.fixes }) {
        Ok(report) => {
            if let Some(why) = report.hru_skipped {
                note(&format!("HRU water balance not simulated ({why}); hru_wb_day.txt has precipitation only"));
            }
            if let Some(why) = report.channel_skipped {
                note(&format!("channels not routed ({why}); they pass their inflow on"));
            }
            if let Some(why) = report.routing_skipped {
                note(&format!("routing units and aquifers not simulated ({why})"));
            }
        }
        Err(e) => return fail(&e),
    }
    let store = match tables.finish() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let names = store.names();
    drop(store);
    for name in &names {
        let txt = out.join(name);
        if o.parquet {
            let stem = name.strip_suffix(".txt").unwrap_or(name);
            let pq = out.join(format!("{stem}.parquet"));
            let r = std::fs::File::open(&txt).map_err(|e| format!("{}: {e}", txt.display())).and_then(|f| {
                let w = std::fs::File::create(&pq).map_err(|e| format!("{}: {e}", pq.display()))?;
                swat_output::parquet::text_to_parquet(std::io::BufReader::new(f), std::io::BufWriter::new(w), 1 << 20)
                    .map_err(|e| format!("{}: {e}", pq.display()))
            });
            match r {
                Ok((rows, _)) => {
                    if json {
                        emit(&format!(
                            "{{\"event\":\"table\",\"file\":{},\"rows\":{rows}}}",
                            swat_output::progress::quote(&format!("{stem}.parquet"))
                        ));
                    }
                }
                Err(e) => return fail(&e),
            }
            if !o.text {
                let _ = std::fs::remove_file(&txt);
            }
        }
        if o.text && json {
            emit(&format!("{{\"event\":\"table\",\"file\":{}}}", swat_output::progress::quote(name)));
        }
    }
    if json {
        emit(&format!("{{\"event\":\"done\",\"days\":{},\"seconds\":{:.3}}}", days.len(), started.elapsed().as_secs_f64()));
    } else {
        println!("wrote the output tables to {}", out.display());
    }
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
