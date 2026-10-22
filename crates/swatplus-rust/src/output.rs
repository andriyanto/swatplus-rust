//! Daily output written as SWAT+ text tables, with the columns the port computes so far.

use std::fmt::Write as _;

use swat_engine::hru::HruDay;
use swat_engine::run::{Day, HruWeather, OutputSink};

/// Collects `hru_wb_day.txt`, `hru_pw_day.txt`, `hru_nb_day.txt` and `hru_ls_day.txt` in SWAT+'s
/// layout. Without the HRU simulation, the water balance has precipitation only, the plant table
/// the weather only, and the other two aren't written.
pub struct TextTables {
    title: String,
    pub wb: String,
    pub pw: String,
    pub nb: String,
    pub ls: String,
}

const KEYS: &str = "  jday   mon   day    yr    unit  gis_id  name            ";

const WEATHER: [&str; 6] = ["tmx", "tmn", "tmpav", "solarad", "wndspd", "rhum"];

impl TextTables {
    pub fn new(title: &str) -> Self {
        Self { title: title.to_string(), wb: String::new(), pw: String::new(), nb: String::new(), ls: String::new() }
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
        if self.wb.is_empty() {
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
    }
}
