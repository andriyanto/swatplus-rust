//! Fortran list-directed input (`read (unit, *) a, b, c`), the way gfortran reads it.
//!
//! Every SWAT+ reader uses it, and its rules decide what a file means:
//!
//! - Each `read` starts at a new record (line). If the record holds fewer values than the read
//!   asks for, it goes on into the next records; values left over in the last record it used are
//!   skipped.
//! - Values are separated by blanks (spaces, tabs) or a comma. Two commas in a row give a null
//!   value, which leaves the variable unchanged.
//! - A slash ends the read: the remaining variables are left unchanged.
//! - `r*c` repeats `c` r times; `r*` is r null values.
//! - Quoted strings ('…' or "…", a doubled quote inside) may hold blanks, commas and slashes.
//!   Unquoted strings end at a blank, comma, slash or end of line.
//! - Blank records are skipped when a read needs values.

use crate::{Error, Result};

/// One value of a read: `None` is a null value (the variable keeps its default).
pub type Item = Option<String>;

/// The records of one file, read the way list-directed `read` statements consume them.
pub struct Records<'a> {
    file: &'a str,
    lines: Vec<&'a str>,
    pos: usize,
}

/// A position to come back to, like Fortran's `backspace` (several records at once).
#[derive(Clone, Copy, Debug)]
pub struct Mark(usize);

impl<'a> Records<'a> {
    /// `file` names the file in errors.
    pub fn new(file: &'a str, text: &'a str) -> Self {
        let lines = text.lines().map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
        Self { file, lines, pos: 0 }
    }

    /// The file's name, as given.
    pub fn file(&self) -> &str {
        self.file
    }

    /// Number (1-based) of the next record a read starts at.
    pub fn line(&self) -> usize {
        self.pos + 1
    }

    /// Whether no record is left.
    pub fn at_end(&self) -> bool {
        self.pos >= self.lines.len()
    }

    pub fn mark(&self) -> Mark {
        Mark(self.pos)
    }

    pub fn reset(&mut self, mark: Mark) {
        self.pos = mark.0.min(self.lines.len());
    }

    /// Reads the next record whole (`read (unit, '(A)')`), blank or not.
    pub fn raw(&mut self) -> Option<&'a str> {
        let line = self.lines.get(self.pos).copied();
        if line.is_some() {
            self.pos += 1;
        }
        line
    }

    /// `read (unit, *) titldum`: reads one value and moves past its record. Blank records before
    /// it are skipped. Returns the value (`None` if null).
    pub fn skip(&mut self) -> Result<Item> {
        let mut items = self.read(1)?;
        Ok(items.items.pop().flatten())
    }

    /// A list-directed read of `n` values. Fewer come back only when a slash ended the read; a
    /// read that reaches the end of the file first is [`Error::Eof`], as `iostat < 0` in SWAT+.
    pub fn read(&mut self, n: usize) -> Result<Fields> {
        let start = self.line();
        let mut items: Vec<Item> = Vec::with_capacity(n);
        if n == 0 {
            // A read of no variables still consumes one record.
            if self.raw().is_none() {
                return Err(self.eof());
            }
            return Ok(Fields { file: self.file.to_string(), line: start, items });
        }
        loop {
            let Some(line) = self.raw() else {
                return Err(self.eof());
            };
            match tokenize(line, n - items.len(), &mut items) {
                Some(Stop::Slash) => break,
                Some(Stop::Full) => break,
                None => {}
            }
            if items.len() >= n {
                break;
            }
        }
        items.truncate(n);
        Ok(Fields { file: self.file.to_string(), line: start, items })
    }

    /// A read of `n` values that keeps what it got when the file ends first, as Fortran assigns
    /// the values read before an end-of-file. `None` when no value was read at all.
    pub fn read_partial(&mut self, n: usize) -> Option<(Fields, bool)> {
        self.read_checked(n, |_, _| true)
    }

    /// [`read_partial`](Self::read_partial) where value `i` must also pass `ok(i, text)` (convert
    /// to its variable's type): a value that doesn't ends the read with the record it is in, as
    /// gfortran's read stops there with an error and the next read starts on the next record.
    pub fn read_checked(&mut self, n: usize, ok: impl Fn(usize, &str) -> bool) -> Option<(Fields, bool)> {
        let start = self.line();
        let mut items: Vec<Item> = Vec::with_capacity(n);
        let mut eof = false;
        loop {
            let Some(line) = self.raw() else {
                eof = true;
                break;
            };
            let first = items.len();
            let stop = tokenize(line, n - items.len(), &mut items);
            let bad = items.iter().enumerate().skip(first).any(|(i, v)| v.as_deref().is_some_and(|t| !ok(i, t)));
            if bad || matches!(stop, Some(Stop::Slash) | Some(Stop::Full)) {
                break;
            }
            if items.len() >= n {
                break;
            }
        }
        if items.is_empty() && eof {
            return None;
        }
        items.truncate(n);
        Some((Fields { file: self.file.to_string(), line: start, items }, eof))
    }

    fn eof(&self) -> Error {
        Error::Eof { file: self.file.to_string() }
    }
}

enum Stop {
    Slash,
    Full,
}

/// Appends up to `want` values of `line` to `out`; tells whether a slash or a full read stopped it.
fn tokenize(line: &str, want: usize, out: &mut Vec<Item>) -> Option<Stop> {
    let target = out.len() + want;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    // A value or a comma was the last thing seen: decides whether a comma is a null value.
    let mut after_value = false;
    while i < chars.len() {
        if out.len() >= target {
            return Some(Stop::Full);
        }
        let Some(&c) = chars.get(i) else { break };
        if c == ' ' || c == '\t' {
            i += 1;
            continue;
        }
        if c == ',' {
            if !after_value {
                out.push(None);
            }
            after_value = false;
            i += 1;
            continue;
        }
        if c == '/' {
            return Some(Stop::Slash);
        }
        if c == '\'' || c == '"' {
            let (text, next) = quoted(&chars, i, c);
            out.push(Some(text));
            i = next;
            after_value = true;
            continue;
        }
        let begin = i;
        while let Some(&ch) = chars.get(i) {
            if ch == ' ' || ch == '\t' || ch == ',' || ch == '/' {
                break;
            }
            i += 1;
        }
        let token: String = chars.get(begin..i).map(|s| s.iter().collect()).unwrap_or_default();
        match repeat(&token) {
            Some((count, value)) => {
                for _ in 0..count {
                    if out.len() >= target {
                        break;
                    }
                    out.push(value.clone());
                }
            }
            None => out.push(Some(token)),
        }
        after_value = true;
    }
    (out.len() >= target).then_some(Stop::Full)
}

/// A quoted string starting at `start`; returns its text and the index after the closing quote.
fn quoted(chars: &[char], start: usize, quote: char) -> (String, usize) {
    let mut text = String::new();
    let mut i = start + 1;
    while let Some(&c) = chars.get(i) {
        if c == quote {
            if chars.get(i + 1) == Some(&quote) {
                text.push(quote);
                i += 2;
                continue;
            }
            return (text, i + 1);
        }
        text.push(c);
        i += 1;
    }
    (text, i)
}

/// `r*c` or `r*`: the count and the repeated value (`None` for nulls). Counts are capped, so a
/// hostile file can't ask for billions of values.
fn repeat(token: &str) -> Option<(usize, Item)> {
    let (count, rest) = token.split_once('*')?;
    if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let count: usize = count.parse().ok()?;
    if count == 0 {
        return None;
    }
    let value = if rest.is_empty() { None } else { Some(rest.to_string()) };
    Some((count.min(100_000), value))
}

/// The values of one read, with typed access. Missing values (a slash ended the read) and null
/// values both read as `None`.
#[derive(Clone, Debug)]
pub struct Fields {
    file: String,
    line: usize,
    pub items: Vec<Item>,
}

impl Fields {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn get(&self, i: usize) -> Option<&str> {
        self.items.get(i).and_then(|v| v.as_deref())
    }

    fn bad(&self, i: usize, what: &str) -> Error {
        Error::Parse {
            file: self.file.clone(),
            line: self.line,
            msg: format!("value {} ({:?}) is not {what}", i + 1, self.get(i).unwrap_or("")),
        }
    }

    /// Value `i` as a string (`None` if null or missing).
    pub fn text(&self, i: usize) -> Option<String> {
        self.get(i).map(str::to_string)
    }

    /// Value `i` as a string, `default` if null or missing.
    pub fn text_or(&self, i: usize, default: &str) -> String {
        self.get(i).unwrap_or(default).to_string()
    }

    /// Value `i` as an integer; Fortran rejects a decimal point here, and so does this.
    pub fn int(&self, i: usize) -> Result<Option<i32>> {
        match self.get(i) {
            None => Ok(None),
            Some(s) => parse_int(s).map(Some).ok_or_else(|| self.bad(i, "an integer")),
        }
    }

    pub fn int_or(&self, i: usize, default: i32) -> Result<i32> {
        Ok(self.int(i)?.unwrap_or(default))
    }

    /// Value `i` as a single-precision real, rounded once from the decimal text, as gfortran
    /// reads a default `real`.
    pub fn real(&self, i: usize) -> Result<Option<f32>> {
        match self.get(i) {
            None => Ok(None),
            Some(s) => parse_real(s).map(Some).ok_or_else(|| self.bad(i, "a number")),
        }
    }

    pub fn real_or(&self, i: usize, default: f32) -> Result<f32> {
        Ok(self.real(i)?.unwrap_or(default))
    }
}

/// A Fortran integer: optional sign and digits.
pub fn parse_int(s: &str) -> Option<i32> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: i64 = s.strip_prefix('+').unwrap_or(s).parse().ok()?;
    i32::try_from(v).ok()
}

/// A Fortran real: `1`, `1.`, `.5`, `1.5e3`, `1.5d3`, `1.5q3`, and the exponent without a letter
/// (`1.5+3`); also `inf` and `nan`.
pub fn parse_real(s: &str) -> Option<f32> {
    let lower = s.to_ascii_lowercase();
    if let Ok(v) = lower.parse::<f32>() {
        return Some(v);
    }
    let mut text: String = lower.replace(['d', 'q'], "e");
    // An exponent sign without its letter: a + or - that isn't first and doesn't follow 'e'.
    let bytes: Vec<u8> = text.bytes().collect();
    if let Some(pos) =
        (1..bytes.len()).find(|&k| matches!(bytes.get(k), Some(b'+') | Some(b'-')) && !matches!(bytes.get(k - 1), Some(b'e')))
    {
        text.insert(pos, 'e');
    }
    let v = text.parse::<f32>().ok()?;
    // Rust also accepts words such as "infinity"; Fortran doesn't take letters but e, d and q.
    if text.chars().any(|c| c.is_ascii_alphabetic() && c != 'e') && v.is_finite() {
        return None;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str, n: usize) -> Vec<Item> {
        Records::new("t", text).read(n).map(|f| f.items).unwrap_or_default()
    }

    fn s(v: &str) -> Item {
        Some(v.to_string())
    }

    #[test]
    fn blanks_tabs_and_commas() {
        assert_eq!(read("1900\t1\t-99\t-99", 4), vec![s("1900"), s("1"), s("-99"), s("-99")]);
        assert_eq!(read("a, b ,c", 3), vec![s("a"), s("b"), s("c")]);
    }

    #[test]
    fn extra_values_are_skipped_and_short_records_continue() {
        let mut r = Records::new("t", "a b c\nd\ne f\n");
        assert_eq!(r.read(2).map(|f| f.items).ok(), Some(vec![s("a"), s("b")]));
        // The read above used one record: the next starts at "d" and goes on into "e f".
        assert_eq!(r.read(2).map(|f| f.items).ok(), Some(vec![s("d"), s("e")]));
        assert!(r.at_end());
    }

    #[test]
    fn blank_records_are_skipped() {
        let mut r = Records::new("t", "\n   \ntitle here\nx");
        assert_eq!(r.skip().ok(), Some(s("title")));
        assert_eq!(r.line(), 4);
    }

    #[test]
    fn nulls_slash_and_repeats() {
        assert_eq!(read("1,,3", 3), vec![s("1"), None, s("3")]);
        assert_eq!(read(",2", 2), vec![None, s("2")]);
        assert_eq!(read("1 2 / 3", 3), vec![s("1"), s("2")]);
        assert_eq!(read("3*0.5 2*", 5), vec![s("0.5"), s("0.5"), s("0.5"), None, None]);
        // Slash ends unquoted text, as in gfortran.
        assert_eq!(read("data/pcp.pcp x", 2), vec![s("data")]);
    }

    #[test]
    fn quoted_strings() {
        assert_eq!(read("'a b, c/d' \"it''s\" 'x''y'", 3), vec![s("a b, c/d"), s("it''s"), s("x'y")]);
    }

    #[test]
    fn end_of_file() {
        let mut r = Records::new("t", "1 2\n");
        assert!(matches!(r.read(3), Err(Error::Eof { .. })));
        let mut r = Records::new("t", "");
        assert!(matches!(r.skip(), Err(Error::Eof { .. })));
    }

    #[test]
    fn crlf_and_mark() {
        let mut r = Records::new("t", "a 1\r\nb 2\r\n");
        let m = r.mark();
        assert_eq!(r.read(2).ok().and_then(|f| f.text(0)), Some("a".into()));
        r.reset(m);
        assert_eq!(r.read(4).map(|f| f.items).ok(), Some(vec![s("a"), s("1"), s("b"), s("2")]));
    }

    #[test]
    fn numbers() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int("+7"), Some(7));
        assert_eq!(parse_int("-3"), Some(-3));
        assert_eq!(parse_int("1.0"), None);
        assert_eq!(parse_int("99999999999"), None);
        assert_eq!(parse_real("1."), Some(1.0));
        assert_eq!(parse_real(".5"), Some(0.5));
        assert_eq!(parse_real("1.5D3"), Some(1500.0));
        assert_eq!(parse_real("1.5+3"), Some(1500.0));
        assert_eq!(parse_real("2.5e-1"), Some(0.25));
        assert_eq!(parse_real("-99"), Some(-99.0));
        assert_eq!(parse_real("abc"), None);
        assert_eq!(parse_real("0.1"), Some(0.1_f32));
        assert!(parse_real("NaN").is_some_and(f32::is_nan));
    }

    #[test]
    fn typed_access() {
        let mut r = Records::new("t", "7 2.5 x");
        let f = r.read(4);
        // Only three values and no next record: end of file.
        assert!(f.is_err());
        let mut r = Records::new("t", "7 2.5 x /");
        let f = r.read(4).ok();
        assert_eq!(f.as_ref().map(Fields::len), Some(3));
        let f = f.unwrap_or_else(|| Fields { file: String::new(), line: 0, items: vec![] });
        assert_eq!(f.int(0).ok().flatten(), Some(7));
        assert_eq!(f.real(1).ok().flatten(), Some(2.5));
        assert!(f.int(2).is_err());
        assert_eq!(f.int_or(3, -1).ok(), Some(-1));
    }
}
