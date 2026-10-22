//! Daily output written as SWAT+ text tables, with the columns the port computes so far.

use std::fmt::Write as _;

use swat_engine::hru::HruWb;
use swat_engine::run::{Day, HruWeather, OutputSink};

/// Collects `hru_wb_day.txt` (the water balance, or only precipitation when the HRUs weren't
/// simulated) and `hru_pw_day.txt` (weather) in SWAT+'s layout.
pub struct TextTables {
    title: String,
    pub wb: String,
    pub pw: String,
}

const KEYS: &str = "  jday   mon   day    yr    unit  gis_id  name            ";

impl TextTables {
    pub fn new(title: &str) -> Self {
        let mut pw = String::new();
        let _ = writeln!(pw, "{title}");
        let _ = writeln!(pw, "{KEYS}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}", "tmx", "tmn", "tmpav", "solarad", "wndspd", "rhum");
        let _ = writeln!(pw, "{:>58}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}", "", "degc", "degc", "degc", "mj/m^2", "m/s", "frac");
        Self { title: title.to_string(), wb: String::new(), pw }
    }

    /// The water balance header, from the first day: all computed columns, or precipitation.
    fn wb_header(&mut self, wb: Option<&HruWb>) {
        let names: Vec<&str> = match wb {
            Some(b) => b.columns().iter().map(|(n, _)| *n).collect(),
            None => vec!["precip"],
        };
        let _ = writeln!(self.wb, "{}", self.title);
        let _ = write!(self.wb, "{KEYS}");
        for n in &names {
            let _ = write!(self.wb, "{n:>12}");
        }
        let _ = write!(self.wb, "\n{:>58}", "");
        for n in &names {
            let _ = write!(self.wb, "{:>12}", if *n == "cn" { "---" } else { "mm" });
        }
        let _ = writeln!(self.wb);
    }
}

fn keys(out: &mut String, d: Day, hru: usize, gis_id: i32, name: &str) {
    let _ = write!(out, "{:6}{:6}{:6}{:6}{:8}{:8}  {:<16}", d.jday, d.mon, d.day, d.yr, hru, gis_id, name);
}

impl OutputSink for TextTables {
    fn hru_day(&mut self, d: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather, wb: Option<&HruWb>) {
        if self.wb.is_empty() {
            self.wb_header(wb);
        }
        keys(&mut self.wb, d, hru, gis_id, name);
        match wb {
            Some(b) => {
                for (_, v) in b.columns() {
                    let _ = write!(self.wb, "{v:12.3}");
                }
                let _ = writeln!(self.wb);
            }
            None => {
                let _ = writeln!(self.wb, "{:12.3}", w.precip);
            }
        }
        keys(&mut self.pw, d, hru, gis_id, name);
        let _ = writeln!(self.pw, "{:12.3}{:12.3}{:12.3}{:12.3}{:12.3}{:12.3}", w.tmax, w.tmin, w.tave, w.solrad, w.windsp, w.rhum);
    }
}
