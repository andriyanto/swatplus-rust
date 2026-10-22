//! `swatplus-rust compare`: compares a SWAT+ daily output table with ours, column by column.
//!
//! Rows are matched by year, day and unit; columns by name, for the columns our file has (the
//! port writes them as it covers them). A value matches when it is within `abs` or within `rel` of
//! the larger of the two. SWAT+ prints three decimals, so the default `abs` is one unit in the last
//! printed place: two values a hair apart can round to neighbouring last digits.

use std::collections::HashMap;

pub struct Table {
    pub names: Vec<String>,
    /// (year, day, unit) → values by column.
    pub rows: HashMap<(i64, i64, i64), Vec<String>>,
    pub order: Vec<(i64, i64, i64)>,
}

/// Reads a SWAT+ output table: a title line, the column names, a units line, then rows.
pub fn read_table(text: &str) -> Result<Table, String> {
    let mut lines = text.lines();
    lines.next();
    let names: Vec<String> = lines.next().ok_or("no header line")?.split_whitespace().map(str::to_string).collect();
    let col = |n: &str| names.iter().position(|x| x == n);
    let (Some(iy), Some(id), Some(iu)) = (col("yr"), col("jday"), col("unit")) else {
        return Err("the header has no yr, jday and unit columns".into());
    };
    let mut rows = HashMap::new();
    let mut order = Vec::new();
    for line in lines {
        let t: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        let key = |i: usize| t.get(i).and_then(|v| v.parse::<i64>().ok());
        let (Some(y), Some(d), Some(u)) = (key(iy), key(id), key(iu)) else { continue };
        order.push((y, d, u));
        rows.insert((y, d, u), t);
    }
    Ok(Table { names, rows, order })
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
pub fn compare(expected: &Table, actual: &Table, abs: f64, rel: f64, label: &str) -> bool {
    let skip = ["jday", "mon", "day", "yr", "unit", "gis_id", "name"];
    let columns: Vec<(usize, usize, &str)> = actual
        .names
        .iter()
        .enumerate()
        .filter(|(_, n)| !skip.contains(&n.as_str()))
        .filter_map(|(ia, n)| expected.names.iter().position(|e| e == n).map(|ie| (ie, ia, n.as_str())))
        .collect();
    let mut stats: Vec<ColumnStats> = columns.iter().map(|_| ColumnStats::default()).collect();
    let mut missing_rows = 0usize;
    for key in &expected.order {
        let Some(a) = actual.rows.get(key) else {
            missing_rows += 1;
            continue;
        };
        let Some(e) = expected.rows.get(key) else { continue };
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
    }
    let extra_rows = actual.order.iter().filter(|k| !expected.rows.contains_key(k)).count();
    println!("{label}: {} rows; columns compared: {}", expected.order.len(), columns.len());
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
    ok
}
