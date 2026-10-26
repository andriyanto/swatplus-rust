//! `swatplus-rust compare`: compares a SWAT+ daily output table with ours, column by column.
//!
//! Rows are matched by year, day and unit; columns by name, for the columns our file has (the
//! port writes them as it covers them). A value matches when it is within `abs` or within `rel` of
//! the larger of the two. SWAT+ prints three decimals, so the default `abs` is one unit in the last
//! printed place: two values a hair apart can round to neighbouring last digits.
//!
//! Both files are read as streams, in step: SWAT+ writes the rows day by day and unit by unit, so
//! the keys (year, day, unit) rise through both, and a large project's tables needn't fit in
//! memory.

use std::io::BufRead;

type Key = (i64, i64, i64);

/// A SWAT+ output table being read: a title line, the column names, a units line, then rows.
struct Rows<R> {
    names: Vec<String>,
    lines: std::io::Lines<R>,
    /// Year, day and unit columns; without a unit column (`ru_day.txt`) rows are numbered
    /// within their day.
    cols: (usize, usize, Option<usize>),
    last: Option<Key>,
}

impl<R: BufRead> Rows<R> {
    fn new(r: R) -> Result<Self, String> {
        let mut lines = r.lines();
        lines.next();
        let header = lines.next().ok_or("no header line")?.map_err(|e| e.to_string())?;
        let names: Vec<String> = header.split_whitespace().map(str::to_string).collect();
        let col = |n: &str| names.iter().position(|x| x == n);
        let (Some(iy), Some(id)) = (col("yr"), col("jday")) else {
            return Err("the header has no yr and jday columns".into());
        };
        let iu = col("unit");
        Ok(Rows { names, lines, cols: (iy, id, iu), last: None })
    }

    /// The next row with a key, or `None` at the end.
    fn next_row(&mut self) -> Result<Option<(Key, Vec<String>)>, String> {
        for line in self.lines.by_ref() {
            let line = line.map_err(|e| e.to_string())?;
            let t: Vec<String> = line.split_whitespace().map(str::to_string).collect();
            let key = |i: usize| t.get(i).and_then(|v| v.parse::<i64>().ok());
            let (Some(y), Some(d)) = (key(self.cols.0), key(self.cols.1)) else { continue };
            let u = match self.cols.2 {
                Some(iu) => match key(iu) {
                    Some(u) => u,
                    None => continue,
                },
                None => match self.last {
                    Some((ly, ld, lu)) if (ly, ld) == (y, d) => lu + 1,
                    _ => 1,
                },
            };
            let k = (y, d, u);
            if self.last.is_some_and(|l| k <= l) {
                return Err(format!("rows out of order at year {y} day {d} unit {u}"));
            }
            self.last = Some(k);
            return Ok(Some((k, t)));
        }
        Ok(None)
    }
}

#[derive(Default)]
struct ColumnStats {
    values: usize,
    mismatches: usize,
    max_abs: f64,
    max_rel: f64,
    first: Option<String>,
}

/// Compares `actual` with `expected`; prints a report and tells whether everything matched.
pub fn compare(expected: impl BufRead, actual: impl BufRead, abs: f64, rel: f64, label: &str) -> Result<bool, String> {
    let mut ex = Rows::new(expected)?;
    let mut ac = Rows::new(actual)?;
    let skip = ["jday", "mon", "day", "yr", "unit", "gis_id", "name", "type"];
    let columns: Vec<(usize, usize, String)> = ac
        .names
        .iter()
        .enumerate()
        .filter(|(_, n)| !skip.contains(&n.as_str()))
        .filter_map(|(ia, n)| ex.names.iter().position(|e| e == n).map(|ie| (ie, ia, n.clone())))
        .collect();
    let mut stats: Vec<ColumnStats> = columns.iter().map(|_| ColumnStats::default()).collect();
    let (mut rows, mut missing_rows, mut extra_rows) = (0usize, 0usize, 0usize);
    let mut e_row = ex.next_row()?;
    let mut a_row = ac.next_row()?;
    loop {
        match (&e_row, &a_row) {
            (None, None) => break,
            (Some(_), None) => {
                rows += 1;
                missing_rows += 1;
                e_row = ex.next_row()?;
            }
            (None, Some(_)) => {
                extra_rows += 1;
                a_row = ac.next_row()?;
            }
            (Some((ke, _)), Some((ka, _))) if ke < ka => {
                rows += 1;
                missing_rows += 1;
                e_row = ex.next_row()?;
            }
            (Some((ke, _)), Some((ka, _))) if ka < ke => {
                extra_rows += 1;
                a_row = ac.next_row()?;
            }
            (Some((key, e)), Some((_, a))) => {
                rows += 1;
                for ((ie, ia, name), s) in columns.iter().zip(stats.iter_mut()) {
                    let x = e.get(*ie).and_then(|v| v.parse::<f64>().ok());
                    let y = a.get(*ia).and_then(|v| v.parse::<f64>().ok());
                    let (Some(x), Some(y)) = (x, y) else { continue };
                    s.values += 1;
                    let d = (x - y).abs();
                    let r = if d > 0.0 { d / x.abs().max(y.abs()) } else { 0.0 };
                    s.max_abs = s.max_abs.max(d);
                    s.max_rel = s.max_rel.max(r);
                    // A hair of slack: 0.144 - 0.143 is a little over 0.001 in binary.
                    if d > abs * (1.0 + 1.0e-6) && r > rel {
                        s.mismatches += 1;
                        if s.first.is_none() {
                            s.first = Some(format!("{} day {} unit {}: SWAT+ {x} vs {y} ({name})", key.0, key.1, key.2));
                        }
                    }
                }
                e_row = ex.next_row()?;
                a_row = ac.next_row()?;
            }
        }
    }
    println!("{label}: {rows} rows; columns compared: {}", columns.len());
    let mut ok = missing_rows == 0 && extra_rows == 0 && !columns.is_empty();
    for ((_, _, name), s) in columns.iter().zip(&stats) {
        let status = if s.mismatches == 0 { "ok" } else { "MISMATCH" };
        println!(
            "  {status:<8} {name:<10} values {:>7}  mismatches {:>6}  max abs {:.4}  max rel {:.2e}",
            s.values, s.mismatches, s.max_abs, s.max_rel
        );
        if let Some(f) = &s.first {
            println!("           first: {f}");
        }
        ok &= s.mismatches == 0;
    }
    if missing_rows > 0 {
        println!("  {missing_rows} SWAT+ rows missing from ours");
    }
    if extra_rows > 0 {
        println!("  {extra_rows} rows of ours not in SWAT+");
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "title\n  jday   mon   day    yr    unit  gis_id  name  a  b\nunits\n";

    #[test]
    fn matches_and_counts_missing_rows() {
        let e = format!("{HEAD}1 1 1 2000 1 1 h1 1.000 2.000\n1 1 1 2000 2 2 h2 1.000 2.000\n2 1 2 2000 1 1 h1 3.000 4.000\n");
        let a = format!("{HEAD}1 1 1 2000 1 1 h1 1.000 2.001\n2 1 2 2000 1 1 h1 3.000 4.000\n");
        assert_eq!(compare(e.as_bytes(), a.as_bytes(), 0.001, 1e-4, "t"), Ok(false));
        let a = format!("{HEAD}1 1 1 2000 1 1 h1 1.000 2.001\n1 1 1 2000 2 2 h2 1.000 2.000\n2 1 2 2000 1 1 h1 3.000 4.000\n");
        assert_eq!(compare(e.as_bytes(), a.as_bytes(), 0.001, 1e-4, "t"), Ok(true));
        let a = format!("{HEAD}1 1 1 2000 1 1 h1 1.000 2.003\n1 1 1 2000 2 2 h2 1.000 2.000\n2 1 2 2000 1 1 h1 3.000 4.000\n");
        assert_eq!(compare(e.as_bytes(), a.as_bytes(), 0.001, 1e-4, "t"), Ok(false));
    }
}
