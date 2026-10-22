//! Daily output written as SWAT+ text tables, with the columns the port computes so far.

use std::fmt::Write as _;

use swat_engine::run::{Day, HruWeather, OutputSink};

/// Collects `hru_wb_day.txt` (precipitation) and `hru_pw_day.txt` (weather) in SWAT+'s layout.
pub struct TextTables {
    pub wb: String,
    pub pw: String,
}

const KEYS: &str = "  jday   mon   day    yr    unit  gis_id  name            ";

impl TextTables {
    pub fn new(title: &str) -> Self {
        let mut wb = String::new();
        let mut pw = String::new();
        let _ = writeln!(wb, "{title}");
        let _ = writeln!(wb, "{KEYS}{:>12}", "precip");
        let _ = writeln!(wb, "{:>58}{:>12}", "", "mm");
        let _ = writeln!(pw, "{title}");
        let _ = writeln!(pw, "{KEYS}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}", "tmx", "tmn", "tmpav", "solarad", "wndspd", "rhum");
        let _ = writeln!(pw, "{:>58}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}", "", "degc", "degc", "degc", "mj/m^2", "m/s", "frac");
        Self { wb, pw }
    }
}

fn keys(out: &mut String, d: Day, hru: usize, gis_id: i32, name: &str) {
    let _ = write!(out, "{:6}{:6}{:6}{:6}{:8}{:8}  {:<16}", d.jday, d.mon, d.day, d.yr, hru, gis_id, name);
}

impl OutputSink for TextTables {
    fn hru_day(&mut self, d: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather) {
        keys(&mut self.wb, d, hru, gis_id, name);
        let _ = writeln!(self.wb, "{:12.3}", w.precip);
        keys(&mut self.pw, d, hru, gis_id, name);
        let _ = writeln!(self.pw, "{:12.3}{:12.3}{:12.3}{:12.3}{:12.3}{:12.3}", w.tmax, w.tmin, w.tave, w.solrad, w.windsp, w.rhum);
    }
}
