//! A small Parquet writer: flat tables of required 32-bit integers, 32-bit floats and UTF-8
//! strings, plain encoded and uncompressed, one data page per column chunk. That is all the
//! output tables need, and any Parquet reader (DuckDB, Arrow, Polars) reads it. The file
//! metadata is Thrift's compact protocol, written by hand: the engine takes no dependencies
//! beyond what its LGPL licence allows.

use std::io::Write;

/// A column's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColType {
    Int32,
    Float,
    Utf8,
}

/// A column's values for one row group.
#[derive(Clone, Debug, PartialEq)]
pub enum Column {
    Int32(Vec<i32>),
    Float(Vec<f32>),
    Utf8(Vec<String>),
}

impl Column {
    pub fn new(t: ColType) -> Column {
        match t {
            ColType::Int32 => Column::Int32(Vec::new()),
            ColType::Float => Column::Float(Vec::new()),
            ColType::Utf8 => Column::Utf8(Vec::new()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Column::Int32(v) => v.len(),
            Column::Float(v) => v.len(),
            Column::Utf8(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn clear(&mut self) {
        match self {
            Column::Int32(v) => v.clear(),
            Column::Float(v) => v.clear(),
            Column::Utf8(v) => v.clear(),
        }
    }

    /// PLAIN encoding: little-endian numbers; a string's length (4 bytes) and its bytes.
    fn plain(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Column::Int32(v) => {
                out.reserve(v.len() * 4);
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
            Column::Float(v) => {
                out.reserve(v.len() * 4);
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
            Column::Utf8(v) => {
                for s in v {
                    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
                    out.extend_from_slice(s.as_bytes());
                }
            }
        }
        out
    }
}

/// Thrift compact protocol, as far as Parquet's metadata needs it.
struct Compact {
    buf: Vec<u8>,
    last: Vec<i16>,
}

const CT_I32: u8 = 5;
const CT_I64: u8 = 6;
const CT_BINARY: u8 = 8;
const CT_LIST: u8 = 9;
const CT_STRUCT: u8 = 12;

impl Compact {
    fn new() -> Self {
        Self { buf: Vec::new(), last: vec![0] }
    }

    fn varint(&mut self, mut v: u64) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.buf.push(b);
                break;
            }
            self.buf.push(b | 0x80);
        }
    }

    fn zigzag(v: i64) -> u64 {
        ((v << 1) ^ (v >> 63)) as u64
    }

    fn field(&mut self, id: i16, t: u8) {
        let last = self.last.last().copied().unwrap_or(0);
        let delta = id - last;
        if delta > 0 && delta <= 15 {
            self.buf.push(((delta as u8) << 4) | t);
        } else {
            self.buf.push(t);
            self.varint(Self::zigzag(i64::from(id)));
        }
        if let Some(l) = self.last.last_mut() {
            *l = id;
        }
    }

    fn i32(&mut self, id: i16, v: i32) {
        self.field(id, CT_I32);
        self.varint(Self::zigzag(i64::from(v)));
    }

    fn i64(&mut self, id: i16, v: i64) {
        self.field(id, CT_I64);
        self.varint(Self::zigzag(v));
    }

    fn string(&mut self, id: i16, s: &str) {
        self.field(id, CT_BINARY);
        self.varint(s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn list_header(&mut self, elem: u8, n: usize) {
        if n < 15 {
            self.buf.push(((n as u8) << 4) | elem);
        } else {
            self.buf.push(0xf0 | elem);
            self.varint(n as u64);
        }
    }

    fn list(&mut self, id: i16, elem: u8, n: usize) {
        self.field(id, CT_LIST);
        self.list_header(elem, n);
    }

    /// A list element that is a struct, or a struct field (after `field(id, CT_STRUCT)`).
    fn begin(&mut self) {
        self.last.push(0);
    }

    fn end(&mut self) {
        self.buf.push(0);
        self.last.pop();
    }

    fn struct_field(&mut self, id: i16) {
        self.field(id, CT_STRUCT);
        self.begin();
    }

    fn list_i32(&mut self, v: i32) {
        self.varint(Self::zigzag(i64::from(v)));
    }

    fn list_string(&mut self, s: &str) {
        self.varint(s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }
}

// Parquet's enums.
const TYPE_INT32: i32 = 1;
const TYPE_FLOAT: i32 = 4;
const TYPE_BYTE_ARRAY: i32 = 6;
const REQUIRED: i32 = 0;
const CONVERTED_UTF8: i32 = 0;
const ENC_PLAIN: i32 = 0;
const ENC_RLE: i32 = 3;
const CODEC_UNCOMPRESSED: i32 = 0;
const PAGE_DATA: i32 = 0;

fn physical(t: ColType) -> i32 {
    match t {
        ColType::Int32 => TYPE_INT32,
        ColType::Float => TYPE_FLOAT,
        ColType::Utf8 => TYPE_BYTE_ARRAY,
    }
}

struct ChunkMeta {
    t: ColType,
    name: String,
    num_values: i64,
    size: i64,
    offset: i64,
}

struct RowGroupMeta {
    chunks: Vec<ChunkMeta>,
    num_rows: i64,
    size: i64,
}

/// Writes a Parquet file to `w`, a row group at a time.
pub struct ParquetWriter<W: Write> {
    w: W,
    pos: u64,
    schema: Vec<(String, ColType)>,
    groups: Vec<RowGroupMeta>,
}

impl<W: Write> ParquetWriter<W> {
    /// Starts the file; column names are made unique (`name`, `name_2`, …).
    pub fn new(mut w: W, schema: &[(String, ColType)]) -> std::io::Result<Self> {
        w.write_all(b"PAR1")?;
        let mut seen: Vec<String> = Vec::new();
        let mut cols = Vec::new();
        for (n, t) in schema {
            let mut name = n.clone();
            let mut k = 2;
            while seen.contains(&name) {
                name = format!("{n}_{k}");
                k += 1;
            }
            seen.push(name.clone());
            cols.push((name, *t));
        }
        Ok(Self { w, pos: 4, schema: cols, groups: Vec::new() })
    }

    pub fn schema(&self) -> &[(String, ColType)] {
        &self.schema
    }

    /// Writes one row group: a column per schema column, all of the same length.
    pub fn write_row_group(&mut self, cols: &[Column]) -> std::io::Result<()> {
        let n = cols.first().map_or(0, Column::len);
        if cols.len() != self.schema.len() || cols.iter().any(|c| c.len() != n) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "columns don't match the schema"));
        }
        if n == 0 {
            return Ok(());
        }
        let mut chunks = Vec::new();
        let mut size = 0_i64;
        for ((name, t), col) in self.schema.iter().zip(cols) {
            let data = col.plain();
            let mut ph = Compact::new();
            ph.i32(1, PAGE_DATA);
            ph.i32(2, data.len() as i32);
            ph.i32(3, data.len() as i32);
            ph.struct_field(5);
            ph.i32(1, n as i32);
            ph.i32(2, ENC_PLAIN);
            ph.i32(3, ENC_RLE);
            ph.i32(4, ENC_RLE);
            ph.end();
            ph.buf.push(0);
            let offset = self.pos as i64;
            self.w.write_all(&ph.buf)?;
            self.w.write_all(&data)?;
            let chunk = (ph.buf.len() + data.len()) as i64;
            self.pos += chunk as u64;
            size += chunk;
            chunks.push(ChunkMeta { t: *t, name: name.clone(), num_values: n as i64, size: chunk, offset });
        }
        self.groups.push(RowGroupMeta { chunks, num_rows: n as i64, size });
        Ok(())
    }

    /// Writes the footer and returns the writer.
    pub fn finish(mut self) -> std::io::Result<W> {
        let mut m = Compact::new();
        m.i32(1, 1);
        m.list(2, CT_STRUCT, self.schema.len() + 1);
        m.begin();
        m.string(4, "schema");
        m.i32(5, self.schema.len() as i32);
        m.end();
        for (name, t) in &self.schema {
            m.begin();
            m.i32(1, physical(*t));
            m.i32(3, REQUIRED);
            m.string(4, name);
            if *t == ColType::Utf8 {
                m.i32(6, CONVERTED_UTF8);
            }
            m.end();
        }
        let num_rows: i64 = self.groups.iter().map(|g| g.num_rows).sum();
        m.i64(3, num_rows);
        m.list(4, CT_STRUCT, self.groups.len());
        for g in &self.groups {
            m.begin();
            m.list(1, CT_STRUCT, g.chunks.len());
            for c in &g.chunks {
                m.begin();
                m.i64(2, c.offset);
                m.struct_field(3);
                m.i32(1, physical(c.t));
                m.list(2, CT_I32, 2);
                m.list_i32(ENC_PLAIN);
                m.list_i32(ENC_RLE);
                m.list(3, CT_BINARY, 1);
                m.list_string(&c.name);
                m.i32(4, CODEC_UNCOMPRESSED);
                m.i64(5, c.num_values);
                m.i64(6, c.size);
                m.i64(7, c.size);
                m.i64(9, c.offset);
                m.end();
                m.end();
            }
            m.i64(2, g.size);
            m.i64(3, g.num_rows);
            m.end();
        }
        m.string(6, concat!("swatplus-rust ", env!("CARGO_PKG_VERSION")));
        m.buf.push(0);
        self.w.write_all(&m.buf)?;
        self.w.write_all(&(m.buf.len() as u32).to_le_bytes())?;
        self.w.write_all(b"PAR1")?;
        self.w.flush()?;
        Ok(self.w)
    }
}

/// The columns SWAT+'s tables key their rows by, as integers.
const INT_COLUMNS: [&str; 9] = ["jday", "mon", "day", "yr", "unit", "gis_id", "tstep", "year", "plant_no"];
/// And those that are text.
const TEXT_COLUMNS: [&str; 5] = ["name", "type", "plantnm", "plant_name", "plnt_name"];

/// A value of a table as text: Fortran's `0.E+00` style and overflows (`****`) too.
fn parse_f32(tok: &str) -> f32 {
    if let Ok(v) = tok.parse::<f32>() {
        return v;
    }
    if tok.contains(".E") || tok.contains(".e") {
        if let Ok(v) = tok.replace(".E", ".0E").replace(".e", ".0e").parse::<f32>() {
            return v;
        }
    }
    f32::NAN
}

/// Converts a SWAT+ text table (`reader`, a line at a time) to Parquet in `w`: the header is
/// the first line after the title that starts with `jday` or `year`, the rows the lines that
/// start with a number (units and blank lines are skipped). Key columns become integers, names
/// text, the rest 32-bit floats. Rows go `group` to a row group. Returns the number of rows.
pub fn text_to_parquet<R: std::io::BufRead, W: Write>(reader: R, w: W, group: usize) -> Result<(usize, W), String> {
    let mut names: Option<Vec<String>> = None;
    let mut first = true;
    let mut pending: Vec<Vec<String>> = Vec::new();
    let mut writer: Option<ParquetWriter<W>> = None;
    let mut w = Some(w);
    let mut cols: Vec<Column> = Vec::new();
    let mut rows = 0_usize;
    let io = |e: std::io::Error| e.to_string();

    // The schema from the header and the first rows.
    fn infer(names: &[String], sample: &[Vec<String>]) -> Vec<(String, ColType)> {
        names
            .iter()
            .enumerate()
            .map(|(k, n)| {
                let lower = n.to_ascii_lowercase();
                let t = if TEXT_COLUMNS.contains(&lower.as_str()) {
                    ColType::Utf8
                } else if INT_COLUMNS.contains(&lower.as_str()) && sample.iter().all(|r| r.get(k).is_some_and(|v| v.parse::<i32>().is_ok()))
                {
                    ColType::Int32
                } else if sample.iter().all(|r| r.get(k).is_none_or(|v| !parse_f32(v).is_nan() || v.to_ascii_lowercase().contains("nan"))) {
                    ColType::Float
                } else {
                    ColType::Utf8
                };
                (n.clone(), t)
            })
            .collect()
    }
    fn push(cols: &mut [Column], row: &[String]) {
        for (k, c) in cols.iter_mut().enumerate() {
            let tok = row.get(k).map(String::as_str).unwrap_or("");
            match c {
                Column::Int32(v) => v.push(tok.parse::<i32>().unwrap_or(0)),
                Column::Float(v) => v.push(if tok.is_empty() { f32::NAN } else { parse_f32(tok) }),
                Column::Utf8(v) => v.push(tok.to_string()),
            }
        }
    }

    for line in reader.lines() {
        let line = line.map_err(io)?;
        if first {
            first = false;
            continue;
        }
        let toks: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        let Some(t0) = toks.first() else { continue };
        if names.is_none() {
            if t0 == "jday" || t0 == "year" {
                names = Some(toks);
            }
            continue;
        }
        if t0.parse::<i64>().is_err() {
            continue;
        }
        if writer.is_none() {
            pending.push(toks);
            if pending.len() >= 1000 {
                let schema = infer(names.as_deref().unwrap_or(&[]), &pending);
                let pw = ParquetWriter::new(w.take().ok_or("writer")?, &schema).map_err(io)?;
                cols = pw.schema().iter().map(|(_, t)| Column::new(*t)).collect();
                for r in pending.drain(..) {
                    push(&mut cols, &r);
                    rows += 1;
                }
                writer = Some(pw);
            }
            continue;
        }
        push(&mut cols, &toks);
        rows += 1;
        if cols.first().map_or(0, Column::len) >= group {
            if let Some(pw) = writer.as_mut() {
                pw.write_row_group(&cols).map_err(io)?;
            }
            cols.iter_mut().for_each(Column::clear);
        }
    }
    if writer.is_none() {
        let schema = infer(names.as_deref().unwrap_or(&[]), &pending);
        let pw = ParquetWriter::new(w.take().ok_or("writer")?, &schema).map_err(io)?;
        cols = pw.schema().iter().map(|(_, t)| Column::new(*t)).collect();
        for r in pending.drain(..) {
            push(&mut cols, &r);
            rows += 1;
        }
        writer = Some(pw);
    }
    let mut pw = writer.ok_or("writer")?;
    pw.write_row_group(&cols).map_err(io)?;
    let w = pw.finish().map_err(io)?;
    Ok((rows, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_field_headers() {
        let mut c = Compact::new();
        c.i32(1, 1);
        c.i32(3, -1);
        c.i32(20, 5);
        assert_eq!(c.buf, vec![0x15, 0x02, 0x25, 0x01, 0x05, 0x28, 0x0a]);
    }

    #[test]
    fn table_to_parquet() {
        let text = " title\n  jday   mon   day    yr    unit  gis_id  name   precip   et\n\n     1     1     1  2015       1      46  hru0046   1.5   0.2E+01\n     2     1     2  2015       1      46  hru0046   0.0   *****\n";
        let (rows, bytes) = text_to_parquet(text.as_bytes(), Vec::new(), 1).unwrap_or_default();
        assert_eq!(rows, 2);
        assert_eq!(&bytes[..4], b"PAR1");
        assert_eq!(&bytes[bytes.len() - 4..], b"PAR1");
    }
}
