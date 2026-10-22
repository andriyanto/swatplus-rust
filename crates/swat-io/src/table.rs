//! Tables SWAT+ reads one derived type per row (`read (unit, *) table(i)`).
//!
//! As the Fortran: the rows are counted first (a list-directed read of one value per record, so
//! blank lines don't count), then that many reads of the whole type follow. A read takes values
//! across lines when a row is short; a value that doesn't convert ends that row's read, leaving
//! the components after it at their defaults; the end of the file ends the table.

use swat_model::table::{Field, Kind, Table, Value, default_row};

use crate::Result;
use crate::list::{Fields, Records, parse_int, parse_real};

/// Reads a table file with the given schema.
pub fn read_table(file: &str, text: &str, fields: &'static [Field]) -> Result<Table> {
    let mut table = Table::empty(file, fields);
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut imax = 0usize;
    while rec.skip().is_ok() {
        imax += 1;
    }
    rec.reset(body);
    let width: usize = fields.iter().map(|f| f.count).sum();
    for _ in 0..imax {
        let Some((items, eof)) = rec.read_partial(width) else { break };
        table.rows.push(fill(fields, &items));
        if eof {
            break;
        }
    }
    Ok(table)
}

/// A row of defaults with the read values put in, up to the first that doesn't convert.
pub fn fill(fields: &[Field], items: &Fields) -> Vec<Value> {
    let mut row = default_row(fields);
    let kinds = fields.iter().flat_map(|f| std::iter::repeat_n(f.kind, f.count));
    for (i, (kind, slot)) in kinds.zip(row.iter_mut()).enumerate() {
        let Some(text) = items.items.get(i) else { break };
        let Some(text) = text else { continue };
        let value = match kind {
            Kind::Text => Some(Value::Text(text.clone())),
            Kind::Int => parse_int(text).map(Value::Int),
            Kind::Real => parse_real(text).map(Value::Real),
        };
        match value {
            Some(v) => *slot = v,
            None => break,
        }
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use swat_model::schema::{CNTABLE, TOPOGRAPHY};

    #[test]
    fn rows_defaults_and_lookup() {
        let text = "topography.hyd: x\nname slp slp_len lat_len dist_cha depos\n\
                    topo1 0.05 60.0 61.0 120.0 0.0\n\n\
                    topo2 0.10\n";
        let t = read_table("topography.hyd", text, TOPOGRAPHY).unwrap();
        // Two rows: the blank line doesn't count; the short last row keeps defaults (end of file).
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.find("topo1"), Some(0));
        assert_eq!(t.real(0, "lat_len"), Some(61.0));
        assert_eq!(t.real(1, "slope"), Some(0.10));
        assert_eq!(t.real(1, "slope_len"), Some(50.0));
        assert_eq!(t.real(1, "dep_co"), Some(1.0));
    }

    #[test]
    fn arrays_and_bad_values() {
        let text = "t\nh\nfal_bare 77 86 x 94 Fallow\nrc 1 2 3 4\n";
        let t = read_table("cntable.lum", text, CNTABLE).unwrap();
        assert_eq!(t.get(0, "cn", 1).and_then(|v| v.as_real()), Some(86.0));
        // "x" doesn't convert: the read stops there, the rest keep defaults.
        assert_eq!(t.get(0, "cn", 2).and_then(|v| v.as_real()), Some(0.0));
        assert_eq!(t.get(1, "cn", 3).and_then(|v| v.as_real()), Some(4.0));
    }
}
