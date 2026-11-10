//! `swatplus-rust batch`: one project run once per parameter set, the runs side by side on the
//! computer's cores, each keeping only the series a calibration needs (`docs/calibration.md`).

use std::fmt::Write as _;
use std::process::ExitCode;

use swat_engine::batch::{Series, run_batch};
use swat_engine::run::RunOptions;
use swat_io::{DirSource, load_project};
use swat_model::cal::{CalUpdate, Calibration};

pub struct BatchArgs<'a> {
    pub sets: Option<&'a str>,
    pub channels: Vec<usize>,
    pub out: Option<&'a str>,
    pub threads: usize,
    pub fixes: bool,
    pub depth: bool,
    pub cal_parms: Option<&'a str>,
}

/// The parameter sets of a CSV file: a header of `parameter:change` columns (`cn2:pctchg`,
/// `esco:absval`), an optional first column `run` naming each row, then a row per run.
pub fn read_sets(text: &str) -> Result<(Vec<String>, Vec<Vec<CalUpdate>>), String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
    let head: Vec<&str> = lines.next().ok_or("the parameter sets file is empty")?.split(',').map(str::trim).collect();
    let named = head.first().is_some_and(|h| h.eq_ignore_ascii_case("run"));
    let cols: Vec<(String, String)> = head
        .iter()
        .skip(usize::from(named))
        .map(|h| {
            let (name, typ) = h.split_once(':').ok_or(format!("column {h}: write it as parameter:change, e.g. cn2:pctchg"))?;
            if !matches!(typ, "absval" | "abschg" | "pctchg" | "relchg") {
                return Err(format!("column {h}: the change is absval, abschg, pctchg or relchg"));
            }
            Ok((name.to_string(), typ.to_string()))
        })
        .collect::<Result<_, String>>()?;
    let (mut names, mut sets) = (Vec::new(), Vec::new());
    for (k, line) in lines.enumerate() {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() != head.len() {
            return Err(format!("row {}: {} values for {} columns", k + 1, f.len(), head.len()));
        }
        names.push(if named { f[0].to_string() } else { format!("{:04}", k + 1) });
        let vals = &f[usize::from(named)..];
        let set = cols
            .iter()
            .zip(vals)
            .map(|((n, t), v)| v.parse::<f32>().map(|v| CalUpdate::all(n, t, v)).map_err(|_| format!("row {}: {v} is not a number", k + 1)))
            .collect::<Result<Vec<_>, String>>()?;
        sets.push(set);
    }
    Ok((names, sets))
}

/// A run's series as CSV: the date, the basin's precipitation and ET, then each channel's outflow
/// (and depths).
pub fn series_csv(s: &Series) -> String {
    let mut o = String::from("date,precip_mm,et_mm");
    for c in &s.channels {
        let _ = write!(o, ",flo_{c}");
    }
    if !s.dep_out.is_empty() {
        for c in &s.channels {
            let _ = write!(o, ",dep_{c},dep_mann_{c}");
        }
    }
    o.push('\n');
    let nd = |y: i32| if swat_model::time::is_leap(y) { swat_model::time::NDAYS_LEAP } else { swat_model::time::NDAYS_NOLEAP };
    for (i, d) in s.days.iter().enumerate() {
        let (mo, dm) = swat_engine::clock::xmon(&nd(d.yr), d.jday);
        let at = |v: &Vec<f32>| v.get(i).copied().unwrap_or(f32::NAN);
        let _ = write!(o, "{:04}-{mo:02}-{dm:02},{},{}", d.yr, at(&s.precip), at(&s.et));
        for f in &s.flo {
            let _ = write!(o, ",{}", at(f));
        }
        for (a, b) in s.dep_out.iter().zip(&s.dep_mann) {
            let _ = write!(o, ",{},{}", at(a), at(b));
        }
        o.push('\n');
    }
    o
}

pub fn batch(dir: &str, a: &BatchArgs) -> ExitCode {
    let fail = |e: &str| {
        eprintln!("error: {e}");
        ExitCode::FAILURE
    };
    let (Some(sets_path), Some(out)) = (a.sets, a.out) else {
        return fail("batch needs --sets <file.csv> and --out <folder>");
    };
    if a.channels.is_empty() {
        return fail("batch needs --channels, e.g. --channels 12,45 (numbers in channel_sd)");
    }
    let project = match load_project(&DirSource::new(dir)) {
        Ok(v) => v.0,
        Err(e) => return fail(&e.to_string()),
    };
    let text = match std::fs::read_to_string(sets_path) {
        Ok(t) => t,
        Err(e) => return fail(&format!("{sets_path}: {e}")),
    };
    let (names, sets) = match read_sets(&text) {
        Ok(v) => v,
        Err(e) => return fail(&format!("{sets_path}: {e}")),
    };
    let mut base = Calibration::of(&project);
    if let Some(path) = a.cal_parms {
        let parsed = std::fs::read_to_string(path)
            .map_err(|e| format!("{path}: {e}"))
            .and_then(|t| swat_io::cal::read_cal_parms(path, &t).map_err(|e| e.to_string()));
        match parsed {
            Ok(p) => base.parms = p,
            Err(e) => return fail(&e),
        }
    }
    if base.parms.is_empty() {
        return fail("the project has no cal_parms.cal; give one with --cal-parms");
    }
    let cals: Vec<Calibration> = sets
        .into_iter()
        .map(|s| Calibration { parms: base.parms.clone(), updates: base.updates.iter().cloned().chain(s).collect() })
        .collect();
    let n_ch = project.objects_of(swat_model::ObjectKind::ChannelDeg).count();
    if let Some(c) = a.channels.iter().find(|&&c| c == 0 || c > n_ch) {
        return fail(&format!("channel {c}: the project has channels 1 to {n_ch}"));
    }
    if let Err(e) = std::fs::create_dir_all(out) {
        return fail(&format!("{out}: {e}"));
    }
    let started = std::time::Instant::now();
    let total = cals.len();
    eprintln!("{total} runs, {} at a time", if a.threads == 0 { "as many as there are cores".to_string() } else { a.threads.to_string() });
    let progress = |done: usize| eprint!("\r{done}/{total} runs done");
    let results = run_batch(&project, RunOptions { fixes: a.fixes, threads: 1 }, &cals, &a.channels, a.depth, a.threads, &progress);
    eprintln!();
    let mut summary = String::from("run,status");
    for c in &a.channels {
        let _ = write!(summary, ",mean_flo_{c}");
    }
    summary.push('\n');
    let mut failed = 0;
    for (name, r) in names.iter().zip(&results) {
        match r {
            Ok((s, _)) => {
                let path = std::path::Path::new(out).join(format!("run_{name}.csv"));
                if let Err(e) = std::fs::write(&path, series_csv(s)) {
                    return fail(&format!("{}: {e}", path.display()));
                }
                let _ = write!(summary, "{name},ok");
                for f in &s.flo {
                    let m = if f.is_empty() { f64::NAN } else { f.iter().map(|v| f64::from(*v)).sum::<f64>() / f.len() as f64 };
                    let _ = write!(summary, ",{m}");
                }
                summary.push('\n');
            }
            Err(e) => {
                failed += 1;
                let _ = writeln!(summary, "{name},\"{}\"", e.replace('"', "'"));
            }
        }
    }
    let path = std::path::Path::new(out).join("runs.csv");
    if let Err(e) = std::fs::write(&path, summary) {
        return fail(&format!("{}: {e}", path.display()));
    }
    eprintln!("{} runs in {:.1} s; {failed} failed; series in {out}", total, started.elapsed().as_secs_f64());
    if failed > 0 { ExitCode::from(3) } else { ExitCode::SUCCESS }
}

#[cfg(test)]
mod tests {
    use super::read_sets;

    #[test]
    fn parameter_sets_are_read_from_csv() {
        let (names, sets) = read_sets("run,cn2:pctchg,esco:absval\n# a comment\na,-10,0.8\nb, 5 ,0.9\n").unwrap_or_default();
        assert_eq!(names, ["a", "b"]);
        assert_eq!(sets[1][0].name, "cn2");
        assert_eq!(sets[1][0].chg_typ, "pctchg");
        assert!((sets[1][0].val - 5.0).abs() < 1e-6);
        assert!((sets[0][1].val - 0.8).abs() < 1e-6);
        // Without a run column the runs are numbered.
        let (names, _) = read_sets("cn2:abschg\n1\n2\n").unwrap_or_default();
        assert_eq!(names, ["0001", "0002"]);
        assert!(read_sets("cn2\n1\n").is_err(), "a column without its change");
        assert!(read_sets("cn2:double\n1\n").is_err(), "an unknown change");
        assert!(read_sets("cn2:pctchg\nx\n").is_err(), "not a number");
        assert!(read_sets("cn2:pctchg,esco:absval\n1\n").is_err(), "a short row");
    }
}
