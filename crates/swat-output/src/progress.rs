//! Progress and events as JSON lines, one object a line on standard output, for a program
//! that runs the engine (RustGIS's runner):
//!
//! ```text
//! {"event":"start","version":"0.1.0","days":1096,"first":"2015-01-01","last":"2017-12-31"}
//! {"event":"progress","done":548,"days":1096,"date":"2016-07-02"}
//! {"event":"note","text":"..."}
//! {"event":"table","file":"hru_wb_day.parquet","rows":126040}
//! {"event":"done","days":1096,"seconds":12.5}
//! {"event":"error","text":"..."}
//! ```

use swat_engine::run::Day;

/// `s` as a JSON string, quotes and all.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A day as `yyyy-mm-dd`.
pub fn date(d: Day) -> String {
    format!("{:04}-{:02}-{:02}", d.yr, d.mon, d.day)
}

/// `{"event":"progress",…}`.
pub fn progress(done: usize, days: usize, d: Day) -> String {
    format!("{{\"event\":\"progress\",\"done\":{done},\"days\":{days},\"date\":\"{}\"}}", date(d))
}

/// An event with a text: `note`, `error`.
pub fn text(event: &str, text: &str) -> String {
    format!("{{\"event\":{},\"text\":{}}}", quote(event), quote(text))
}

/// Reports progress through `emit` when the share done reaches a new whole percent, and at
/// the end.
pub struct Reporter {
    last_pct: Option<usize>,
    emit: Box<dyn FnMut(&str)>,
}

impl Reporter {
    pub fn new(emit: Box<dyn FnMut(&str)>) -> Self {
        Self { last_pct: None, emit }
    }

    pub fn day(&mut self, done: usize, days: usize, d: Day) {
        let pct = (done * 100).checked_div(days).unwrap_or(100);
        if self.last_pct != Some(pct) || done == days {
            self.last_pct = Some(pct);
            (self.emit)(&progress(done, days, d));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json() {
        assert_eq!(quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
        assert_eq!(
            progress(1, 2, Day { jday: 1, mon: 1, day: 1, yr: 2015 }),
            "{\"event\":\"progress\",\"done\":1,\"days\":2,\"date\":\"2015-01-01\"}"
        );
    }
}
