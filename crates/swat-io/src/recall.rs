//! `recall.rec` and the files it names (`recall_read`).

use swat_model::{RecallDb, RecallRow};

use crate::list::Records;
use crate::source::FileSource;
use crate::{Error, Result};

/// Reads `recall.rec` (`name`, `text`) and each recall's file from `src`. An export coefficient
/// (type 4) has no file of its own.
pub fn read_recalls(src: &dyn FileSource, name: &str, text: &str) -> Result<Vec<RecallDb>> {
    let mut out = Vec::new();
    let mut rec = Records::new(name, text);
    rec.skip()?;
    rec.skip()?;
    loop {
        let f = match rec.read(4) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let id = f.int_or(0, 0)?.max(0) as usize;
        let mut db = RecallDb { id, name: f.text_or(1, ""), typ: f.int_or(2, 0)?, file: f.text_or(3, ""), rows: Vec::new() };
        if db.typ != 4 {
            // A file that isn't there (Fortran's open makes it, empty) or ends before its
            // header ends the reading of recall.rec there.
            let text = src.read(&db.file)?.unwrap_or_default();
            match read_rows(&db.file, &text)? {
                Some(rows) => db.rows = rows,
                None => break,
            }
        }
        out.push(db);
    }
    Ok(out)
}

/// A recall file: a title, the number of years, a header, then a record a time step: day of the
/// year, month, day, year, object type and name, and a hydrograph's 18 values.
fn read_rows(file: &str, text: &str) -> Result<Option<Vec<RecallRow>>> {
    let mut rec = Records::new(file, text);
    for _ in 0..3 {
        if rec.skip().is_err() {
            return Ok(None);
        }
    }
    let mut rows = Vec::new();
    loop {
        let f = match rec.read(24) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let mut vals = [0.0_f32; 18];
        for (k, v) in vals.iter_mut().enumerate() {
            *v = f.real_or(6 + k, 0.0)?;
        }
        rows.push(RecallRow { jday: f.int_or(0, 0)?, mo: f.int_or(1, 0)?, day_mo: f.int_or(2, 0)?, iyr: f.int_or(3, 0)?, vals });
    }
    Ok(Some(rows))
}
