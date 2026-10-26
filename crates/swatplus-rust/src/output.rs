//! Output written as SWAT+ text tables (`<table>_day.txt`, `_mon`, `_yr`, `_aa`), with the
//! columns the port computes, for the tables and periods `print.prt` asks for.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use swat_engine::channel::ChWat;
use swat_engine::hru::HruDay;
use swat_engine::hyd::Hyd;
use swat_engine::periods::{PW_NAMES, hru_values};
use swat_engine::route::AquDay;
use swat_engine::run::{Day, HruRows, HruWeather, OutputSink, Period, Region};
use swat_model::PrintSettings;

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

/// What a file's buffer holds before it is written.
const CHUNK: usize = 4 << 20;

const KEYS: &str = "  jday   mon   day    yr    unit  gis_id  name            ";

const WEATHER: [&str; 6] = ["tmx", "tmn", "tmpav", "solarad", "wndspd", "rhum"];

/// The tables the port writes, as `print.prt` names them, and their file name stems.
const TABLES: [&str; 17] = [
    "hru_wb",
    "hru_nb",
    "hru_ls",
    "hru_pw",
    "ru",
    "aquifer",
    "channel_sd",
    "basin_wb",
    "basin_nb",
    "basin_ls",
    "basin_pw",
    "lsunit_wb",
    "lsunit_nb",
    "lsunit_ls",
    "lsunit_pw",
    "basin_aqu",
    "basin_sd_cha",
];

#[derive(Default)]
struct Sheet {
    buf: String,
    file: Option<File>,
    started: bool,
}

/// Collects the tables into folder `dir`, a few megabytes at a time (a large project's daily
/// tables run to gigabytes). Without the HRU simulation, the daily water balance has
/// precipitation only, the plant table the weather only, and the other HRU tables aren't
/// written.
pub struct TextTables {
    title: String,
    dir: PathBuf,
    sheets: BTreeMap<String, Sheet>,
    /// Per table, whether `print.prt` asks for its daily, monthly, yearly, average annual file.
    flags: BTreeMap<&'static str, [bool; 4]>,
    error: Option<String>,
    /// Correct SWAT+'s mislabelled aquifer columns (`--fix-swat`).
    pub fixes: bool,
    /// `crop_yld`: `y` the basin's crops, `a` each HRU's, `b` both.
    crop_yld: String,
}

fn period_index(p: Option<Period>) -> usize {
    match p {
        None => 0,
        Some(Period::Mon) => 1,
        Some(Period::Yr) => 2,
        Some(Period::Aa) => 3,
    }
}

fn suffix(p: Option<Period>) -> &'static str {
    match p {
        None => "day",
        Some(p) => p.suffix(),
    }
}

impl TextTables {
    pub fn new(title: &str, dir: &Path, print: &PrintSettings) -> Self {
        let mut flags = BTreeMap::new();
        for name in TABLES {
            let f = match print.object(name) {
                Some(o) => [o.daily == "y", o.monthly == "y", o.yearly == "y", o.avann == "y"],
                None => [false; 4],
            };
            flags.insert(name, f);
        }
        Self {
            title: title.to_string(),
            dir: dir.to_path_buf(),
            sheets: BTreeMap::new(),
            flags,
            error: None,
            fixes: false,
            crop_yld: print.crop_yld.clone(),
        }
    }

    fn wants(&self, table: &str, p: Option<Period>) -> bool {
        self.flags.get(table).is_some_and(|f| f[period_index(p)])
    }

    /// The buffer of file `name`, with its header written by `header` the first time.
    fn file(&mut self, name: &str, header: impl FnOnce(&mut String)) -> &mut String {
        let s = self.sheets.entry(name.to_string()).or_default();
        if !s.started {
            s.started = true;
            header(&mut s.buf);
        }
        &mut s.buf
    }

    /// The buffer of `<stem>_<period>.txt`, with its header written by `header` the first time.
    fn sheet(&mut self, stem: &str, p: Option<Period>, header: impl FnOnce(&mut String)) -> &mut String {
        let key = format!("{stem}_{}.txt", suffix(p));
        let s = self.sheets.entry(key).or_default();
        if !s.started {
            s.started = true;
            header(&mut s.buf);
        }
        &mut s.buf
    }

    /// Writes buffers over `limit` bytes to their files, creating them on the first write.
    fn flush(&mut self, limit: usize) {
        for (name, s) in self.sheets.iter_mut() {
            if s.buf.len() <= limit || self.error.is_some() {
                continue;
            }
            let path = self.dir.join(name);
            if s.file.is_none() {
                match File::create(&path) {
                    Ok(f) => s.file = Some(f),
                    Err(e) => {
                        self.error = Some(format!("{}: {e}", path.display()));
                        continue;
                    }
                }
            }
            if let Some(f) = s.file.as_mut()
                && let Err(e) = f.write_all(s.buf.as_bytes())
            {
                self.error = Some(format!("{}: {e}", path.display()));
            }
            s.buf.clear();
        }
    }

    /// Writes the rest; `Err` names the file that couldn't be written.
    pub fn finish(mut self) -> Result<(), String> {
        for s in self.sheets.values_mut() {
            // A table with a header and no rows is still written.
            if s.buf.is_empty() && s.file.is_none() {
                s.buf.push('\n');
            }
        }
        self.flush(0);
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn hru_rows(&mut self, prefix: &str, p: Option<Period>, d: Day, hru: usize, gis_id: i32, name: &str, r: &HruRows, nutrients: bool) {
        let title = self.title.clone();
        let t = |s: &str| format!("{prefix}_{s}");
        let wb_names = swat_engine::hru::HruWb::default().columns().map(|(n, _)| n);
        let nb_names = swat_engine::hru::HruNb::default().columns().map(|(n, _)| n);
        let ls_names = swat_engine::hru::HruLs::default().columns().map(|(n, _)| n);
        if self.wants(&t("wb"), p) {
            let out = self.sheet(&t("wb"), p, |o| header(o, &title, &wb_names));
            keys(out, d, hru, gis_id, name);
            row(out, r.wb);
        }
        let pw: Vec<(&str, f32)> = PW_NAMES.iter().copied().zip(r.pw).filter(|(n, _)| nutrients || *n != "percn").collect();
        if self.wants(&t("pw"), p) {
            let names: Vec<&str> = pw.iter().map(|(n, _)| *n).collect();
            let out = self.sheet(&t("pw"), p, |o| header(o, &title, &names));
            keys(out, d, hru, gis_id, name);
            row(out, pw.iter().map(|(_, v)| *v));
        }
        if nutrients && self.wants(&t("nb"), p) {
            let out = self.sheet(&t("nb"), p, |o| header(o, &title, &nb_names));
            keys(out, d, hru, gis_id, name);
            row(out, r.nb);
        }
        if nutrients && self.wants(&t("ls"), p) {
            let out = self.sheet(&t("ls"), p, |o| header(o, &title, &ls_names));
            keys(out, d, hru, gis_id, name);
            row(out, r.ls);
        }
    }

    fn ru_row(&mut self, p: Option<Period>, d: Day, name: &str, hd: &Hyd) {
        if !self.wants("ru", p) {
            return;
        }
        let title = self.title.clone();
        let out = self.sheet("ru", p, |o| {
            let _ = writeln!(o, "{title}");
            let _ = write!(o, "{:>6}{:>6}{:>6}{:>6}  {:<16}{:>6}", "jday", "mon", "day", "yr", "name", "type");
            for (n, _) in hd.columns() {
                let _ = write!(o, " {n:>15}");
            }
            let _ = writeln!(o);
            let _ = writeln!(o);
        });
        let _ = write!(out, "{:6}{:6}{:6}{:6}  {:<16}{:>6}", d.jday, d.mon, d.day, d.yr, name, "ru");
        for (_, v) in hd.columns() {
            let _ = write!(out, " {v:15.8e}");
        }
        let _ = writeln!(out);
    }

    #[allow(clippy::too_many_arguments)]
    fn cha_row(
        &mut self,
        stem: &str,
        p: Option<Period>,
        d: Day,
        ich: usize,
        gis_id: i32,
        name: &str,
        w: &ChWat,
        stor: &Hyd,
        hin: &Hyd,
        hout: &Hyd,
        wtemp: Option<f32>,
    ) {
        if !self.wants(stem, p) {
            return;
        }
        let title = self.title.clone();
        let out = self.sheet(stem, p, |o| {
            let _ = writeln!(o, "{title}");
            let _ = write!(o, "{KEYS}");
            for n in ["area", "precip", "evap", "seep"] {
                let _ = write!(o, " {n:>14}");
            }
            for suffix in ["_stor", "_in", "_out"] {
                for (n, _) in stor.columns() {
                    let n = if n == "null" { n.to_string() } else { format!("{n}{suffix}") };
                    let _ = write!(o, " {n:>14}");
                }
            }
            if wtemp.is_some() {
                let _ = write!(o, " {:>14}", "water_temp");
            }
            let _ = writeln!(o);
            let _ = writeln!(o);
        });
        keys(out, d, ich, gis_id, name);
        // basin_sdchannel_output prints the area as f16.4.
        if wtemp.is_none() {
            let _ = write!(out, "{:16.4}", w.area_ha);
        } else {
            out.push_str(&e15_4(w.area_ha));
        }
        for v in [w.precip, w.evap, w.seep] {
            out.push_str(&e15_4(v));
        }
        for h in [stor, hin, hout] {
            for (_, v) in h.columns() {
                out.push_str(&e15_4(v));
            }
        }
        if let Some(t) = wtemp {
            out.push_str(&e15_4(t));
        }
        out.push('\n');
    }

    #[allow(clippy::too_many_arguments)]
    fn aqu_row(&mut self, stem: &str, p: Option<Period>, d: Day, iaq: usize, gis_id: i32, name: &str, a: &AquDay) {
        if !self.wants(stem, p) {
            return;
        }
        let (title, fixes) = (self.title.clone(), self.fixes);
        let out = self.sheet(stem, p, |o| {
            let _ = writeln!(o, "{title}");
            let _ = write!(o, "{KEYS}");
            for (n, _) in a.columns_as(fixes) {
                let _ = write!(o, " {n:>14}");
            }
            let _ = writeln!(o);
            let _ = writeln!(o);
        });
        keys(out, d, iaq, gis_id, name);
        for (_, v) in a.columns() {
            let _ = write!(out, "{v:15.3}");
        }
        let _ = writeln!(out);
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

impl OutputSink for TextTables {
    fn hru_day(&mut self, d: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather, h: Option<&HruDay>) {
        let weather = [w.tmax, w.tmin, w.tave, w.solrad, w.windsp, w.rhum];
        match h {
            Some(h) => {
                let (wb, nb, ls, pw) = hru_values(h, weather);
                self.hru_rows("hru", None, d, hru, gis_id, name, &HruRows { wb, nb, ls, pw }, h.nutrients);
            }
            None => {
                let title = self.title.clone();
                if self.wants("hru_wb", None) {
                    let out = self.sheet("hru_wb", None, |o| header(o, &title, &["precip"]));
                    keys(out, d, hru, gis_id, name);
                    row(out, [w.precip]);
                }
                if self.wants("hru_pw", None) {
                    let out = self.sheet("hru_pw", None, |o| header(o, &title, &WEATHER));
                    keys(out, d, hru, gis_id, name);
                    row(out, weather);
                }
            }
        }
        self.flush(CHUNK);
    }

    fn hru_period(&mut self, p: Period, d: Day, hru: usize, gis_id: i32, name: &str, rows: &HruRows, nutrients: bool) {
        self.hru_rows("hru", Some(p), d, hru, gis_id, name, rows, nutrients);
        self.flush(CHUNK);
    }

    fn ru_day(&mut self, d: Day, name: &str, hd: &Hyd) {
        self.ru_row(None, d, name, hd);
        self.flush(CHUNK);
    }

    fn ru_period(&mut self, p: Period, d: Day, name: &str, hd: &Hyd) {
        self.ru_row(Some(p), d, name, hd);
    }

    fn cha_day(&mut self, d: Day, ich: usize, gis_id: i32, name: &str, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd, wtemp: f32) {
        self.cha_row("channel_sd", None, d, ich, gis_id, name, w, stor, hin, hout, Some(wtemp));
        self.flush(CHUNK);
    }

    fn cha_period(
        &mut self,
        p: Period,
        d: Day,
        ich: usize,
        gis_id: i32,
        name: &str,
        w: &ChWat,
        stor: &Hyd,
        hin: &Hyd,
        hout: &Hyd,
        wtemp: f32,
    ) {
        self.cha_row("channel_sd", Some(p), d, ich, gis_id, name, w, stor, hin, hout, Some(wtemp));
    }

    fn aqu_day(&mut self, d: Day, iaq: usize, gis_id: i32, name: &str, a: &AquDay) {
        self.aqu_row("aquifer", None, d, iaq, gis_id, name, a);
        self.flush(CHUNK);
    }

    fn aqu_period(&mut self, p: Period, d: Day, iaq: usize, gis_id: i32, name: &str, a: &AquDay) {
        self.aqu_row("aquifer", Some(p), d, iaq, gis_id, name, a);
    }

    fn region(&mut self, r: Region, p: Option<Period>, d: Day, unit: usize, gis_id: i32, name: &str, rows: &HruRows) {
        let prefix = match r {
            Region::Basin => "basin",
            Region::Lsu => "lsunit",
        };
        self.hru_rows(prefix, p, d, unit, gis_id, name, rows, true);
        self.flush(CHUNK);
    }

    fn crop_yld_aa(&mut self, d: Day, hru: usize, plant: &str, y: [f32; 4]) {
        if !matches!(self.crop_yld.as_str(), "a" | "b") {
            return;
        }
        let title = self.title.clone();
        let out = self.file("crop_yld_aa.txt", |o| {
            let _ = writeln!(o, "{title}");
            let _ = writeln!(o, "{:>93}", "--YIELD (kg/ha)--");
            let _ = writeln!(o, "  jday   mon   day    yr    unit PLANTNM                         MASS          C           N           P");
        });
        let _ = write!(out, "{:6}{:6}{:6}{:6}{:8}    {:<40}     ", d.jday, d.mon, d.day, d.yr, hru, plant);
        for v in y {
            let _ = write!(out, "{v:12.3}");
        }
        let _ = writeln!(out);
    }

    fn basin_crop_yld(&mut self, p: Period, yrc: i32, iplt: usize, plant: &str, area: f32, yld: f32, t_ha: f32) {
        if !matches!(self.crop_yld.as_str(), "y" | "b") {
            return;
        }
        let title = self.title.clone();
        let name = format!("basin_crop_yld_{}.txt", p.suffix());
        let out = self.file(&name, |o| {
            let _ = writeln!(o, "{title}");
            let _ = writeln!(o, "       year      plant_no   plant_name       harv_area(ha)     yld(t)          yld(t/ha)      ");
        });
        let _ = writeln!(out, "{yrc:12}{iplt:12} {plant:<40}{area:17.9}{yld:17.9}{t_ha:17.9}");
    }

    fn basin_aqu(&mut self, p: Option<Period>, d: Day, name: &str, a: &AquDay) {
        self.aqu_row("basin_aqu", p, d, 1, 1, name, a);
    }

    fn basin_cha(&mut self, p: Option<Period>, d: Day, name: &str, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd) {
        self.cha_row("basin_sd_cha", p, d, 1, 1, name, w, stor, hin, hout, None);
    }
}
