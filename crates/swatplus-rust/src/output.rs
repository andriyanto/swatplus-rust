//! Daily output written as SWAT+ text tables, with the columns the port computes so far.

use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use swat_engine::hru::HruDay;
use swat_engine::run::{Day, HruWeather, OutputSink};

/// Collects `hru_wb_day.txt`, `hru_pw_day.txt`, `hru_nb_day.txt` and `hru_ls_day.txt` in SWAT+'s
/// layout into folder `dir`, a few megabytes at a time (a large project's tables run to
/// gigabytes). Without the HRU simulation, the water balance has precipitation only, the plant
/// table the weather only, and the other two aren't written.
pub struct TextTables {
    title: String,
    dir: PathBuf,
    wb: String,
    pw: String,
    nb: String,
    ls: String,
    ru: String,
    aqu: String,
    cha: String,
    files: [Option<File>; 7],
    started: bool,
    ru_started: bool,
    aqu_started: bool,
    cha_started: bool,
    error: Option<String>,
    /// Correct SWAT+'s mislabelled aquifer columns (`--fix-swat`).
    pub fixes: bool,
}

const FILES: [&str; 7] =
    ["hru_wb_day.txt", "hru_pw_day.txt", "hru_nb_day.txt", "hru_ls_day.txt", "ru_day.txt", "aquifer_day.txt", "channel_sd_day.txt"];

/// Fortran `Ew.d` with `d` 4 and width 15: `0.dddd` and a two-digit exponent.
fn e15_4(v: f32) -> String {
    if v == 0.0 {
        return format!("{:>15}", "0.0000E+00");
    }
    let s = format!("{:.3e}", v.abs());
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let e: i32 = e.parse::<i32>().unwrap_or(0) + 1;
    let digits: String = m.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if v < 0.0 { "-" } else { "" };
    let es = if e < 0 { '-' } else { '+' };
    format!("{:>15}", format!("{sign}0.{digits}E{es}{:02}", e.abs()))
}

/// What the buffers hold before they go to their files.
const CHUNK: usize = 8 << 20;

const KEYS: &str = "  jday   mon   day    yr    unit  gis_id  name            ";

const WEATHER: [&str; 6] = ["tmx", "tmn", "tmpav", "solarad", "wndspd", "rhum"];

impl TextTables {
    pub fn new(title: &str, dir: &Path) -> Self {
        Self {
            title: title.to_string(),
            dir: dir.to_path_buf(),
            wb: String::new(),
            pw: String::new(),
            nb: String::new(),
            ls: String::new(),
            ru: String::new(),
            aqu: String::new(),
            cha: String::new(),
            files: [None, None, None, None, None, None, None],
            started: false,
            ru_started: false,
            aqu_started: false,
            cha_started: false,
            error: None,
            fixes: false,
        }
    }

    /// Writes what the buffers hold to the files, creating them on the first write.
    fn flush(&mut self) {
        let bufs = [&mut self.wb, &mut self.pw, &mut self.nb, &mut self.ls, &mut self.ru, &mut self.aqu, &mut self.cha];
        for ((buf, file), name) in bufs.into_iter().zip(self.files.iter_mut()).zip(FILES) {
            if buf.is_empty() || self.error.is_some() {
                continue;
            }
            let path = self.dir.join(name);
            if file.is_none() {
                match File::create(&path) {
                    Ok(f) => *file = Some(f),
                    Err(e) => {
                        self.error = Some(format!("{}: {e}", path.display()));
                        continue;
                    }
                }
            }
            if let Some(f) = file.as_mut()
                && let Err(e) = f.write_all(buf.as_bytes())
            {
                self.error = Some(format!("{}: {e}", path.display()));
            }
            buf.clear();
        }
    }

    /// Writes the rest; `Err` names the file that couldn't be written.
    pub fn finish(mut self) -> Result<(), String> {
        self.flush();
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

fn header(out: &mut String, title: &str, names: &[&str]) {
    let _ = writeln!(out, "{title}");
    let _ = write!(out, "{KEYS}");
    for n in names {
        let _ = write!(out, " {n:>11}");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "{:>58}", "");
}

fn keys(out: &mut String, d: Day, hru: usize, gis_id: i32, name: &str) {
    let _ = write!(out, "{:6}{:6}{:6}{:6}{:8}{:8}  {:<16}", d.jday, d.mon, d.day, d.yr, hru, gis_id, name);
}

fn row(out: &mut String, values: impl IntoIterator<Item = f32>) {
    for v in values {
        let _ = write!(out, "{v:12.3}");
    }
    let _ = writeln!(out);
}

/// The plant columns, without nitrate percolation when the nutrients aren't simulated.
fn pw_columns(h: &HruDay) -> Vec<(&'static str, f32)> {
    h.pw.columns().into_iter().filter(|(n, _)| h.nutrients || *n != "percn").collect()
}

impl OutputSink for TextTables {
    fn hru_day(&mut self, d: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather, h: Option<&HruDay>) {
        let weather = [w.tmax, w.tmin, w.tave, w.solrad, w.windsp, w.rhum];
        if !self.started {
            self.started = true;
            match h {
                Some(h) => {
                    let names: Vec<&str> = h.wb.columns().iter().map(|(n, _)| *n).collect();
                    header(&mut self.wb, &self.title, &names);
                    let mut names: Vec<&str> = pw_columns(h).iter().map(|(n, _)| *n).collect();
                    names.extend(WEATHER);
                    header(&mut self.pw, &self.title, &names);
                    if h.nutrients {
                        let names: Vec<&str> = h.nb.columns().iter().map(|(n, _)| *n).collect();
                        header(&mut self.nb, &self.title, &names);
                        let names: Vec<&str> = h.ls.columns().iter().map(|(n, _)| *n).collect();
                        header(&mut self.ls, &self.title, &names);
                    }
                }
                None => {
                    header(&mut self.wb, &self.title, &["precip"]);
                    header(&mut self.pw, &self.title, &WEATHER);
                }
            }
        }
        keys(&mut self.wb, d, hru, gis_id, name);
        keys(&mut self.pw, d, hru, gis_id, name);
        match h {
            Some(h) => {
                row(&mut self.wb, h.wb.columns().map(|(_, v)| v));
                row(&mut self.pw, pw_columns(h).into_iter().map(|(_, v)| v).chain(weather));
                if h.nutrients {
                    keys(&mut self.nb, d, hru, gis_id, name);
                    row(&mut self.nb, h.nb.columns().map(|(_, v)| v));
                    keys(&mut self.ls, d, hru, gis_id, name);
                    row(&mut self.ls, h.ls.columns().map(|(_, v)| v));
                }
            }
            None => {
                row(&mut self.wb, [w.precip]);
                row(&mut self.pw, weather);
            }
        }
        if self.wb.len() + self.pw.len() + self.nb.len() + self.ls.len() > CHUNK {
            self.flush();
        }
    }

    fn ru_day(&mut self, d: Day, name: &str, hd: &swat_engine::hyd::Hyd) {
        if !self.ru_started {
            self.ru_started = true;
            let _ = writeln!(self.ru, "{}", self.title);
            let _ = write!(self.ru, "{:>6}{:>6}{:>6}{:>6}  {:<16}{:>6}", "jday", "mon", "day", "yr", "name", "type");
            for (n, _) in hd.columns() {
                let _ = write!(self.ru, " {n:>15}");
            }
            let _ = writeln!(self.ru);
            let _ = writeln!(self.ru);
        }
        let _ = write!(self.ru, "{:6}{:6}{:6}{:6}  {:<16}{:>6}", d.jday, d.mon, d.day, d.yr, name, "ru");
        for (_, v) in hd.columns() {
            let _ = write!(self.ru, " {v:15.8e}");
        }
        let _ = writeln!(self.ru);
        if self.ru.len() > CHUNK {
            self.flush();
        }
    }

    fn cha_day(
        &mut self,
        d: Day,
        ich: usize,
        gis_id: i32,
        name: &str,
        w: &swat_engine::channel::ChWat,
        stor: &swat_engine::hyd::Hyd,
        hin: &swat_engine::hyd::Hyd,
        hout: &swat_engine::hyd::Hyd,
        wtemp: f32,
    ) {
        if !self.cha_started {
            self.cha_started = true;
            let _ = writeln!(self.cha, "{}", self.title);
            let _ = write!(self.cha, "{KEYS}");
            for n in ["area", "precip", "evap", "seep"] {
                let _ = write!(self.cha, " {n:>14}");
            }
            for suffix in ["_stor", "_in", "_out"] {
                for (n, _) in stor.columns() {
                    let n = if n == "null" { n.to_string() } else { format!("{n}{suffix}") };
                    let _ = write!(self.cha, " {n:>14}");
                }
            }
            let _ = writeln!(self.cha, " {:>14}", "water_temp");
            let _ = writeln!(self.cha);
        }
        keys(&mut self.cha, d, ich, gis_id, name);
        for v in [w.area_ha, w.precip, w.evap, w.seep] {
            self.cha.push_str(&e15_4(v));
        }
        for h in [stor, hin, hout] {
            for (_, v) in h.columns() {
                self.cha.push_str(&e15_4(v));
            }
        }
        self.cha.push_str(&e15_4(wtemp));
        self.cha.push('\n');
        if self.cha.len() > CHUNK {
            self.flush();
        }
    }

    fn aqu_day(&mut self, d: Day, iaq: usize, gis_id: i32, name: &str, a: &swat_engine::route::AquDay) {
        if !self.aqu_started {
            self.aqu_started = true;
            let _ = writeln!(self.aqu, "{}", self.title);
            let _ = write!(self.aqu, "{KEYS}");
            for (n, _) in a.columns_as(self.fixes) {
                let _ = write!(self.aqu, " {n:>14}");
            }
            let _ = writeln!(self.aqu);
            let _ = writeln!(self.aqu);
        }
        keys(&mut self.aqu, d, iaq, gis_id, name);
        for (_, v) in a.columns() {
            let _ = write!(self.aqu, "{v:15.3}");
        }
        let _ = writeln!(self.aqu);
        if self.aqu.len() > CHUNK {
            self.flush();
        }
    }
}
