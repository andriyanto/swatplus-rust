//! `file.cio` (SWAT+ `readcio_read`).

use swat_model::cio::{FileCio, SCHEMA};

use crate::list::Records;
use crate::{Error, Result};

/// Reads `file.cio`. Rows are read by position, not by the name at their start; a row that is
/// missing (end of file) keeps its defaults, a `null` turns its slot off, and a row with more names
/// than slots has the extra ones ignored.
pub fn read(text: &str) -> Result<FileCio> {
    let mut cio = FileCio::defaults();
    let mut rec = Records::new("file.cio", text);
    cio.title = rec.skip()?.unwrap_or_default();
    for (section, (_, slots)) in cio.sections.iter_mut().zip(SCHEMA.iter()) {
        let fields = match rec.read(1 + slots.len()) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => return Ok(cio),
            Err(e) => return Err(e),
        };
        section.name_in_file = fields.text(0);
        for (i, (_, file)) in section.files.iter_mut().enumerate() {
            // A null value (",,") or a value cut off by a slash leaves the default.
            if let Some(value) = fields.text(i + 1) {
                *file = if value == "null" { None } else { Some(value) };
            }
        }
    }
    // The line after the weather paths: the output folder, the text after its first word.
    if let Some(line) = rec.raw() {
        let line = line.trim_start();
        cio.out_path = line.split_once(' ').map(|(_, rest)| rest.trim().to_string()).filter(|s| !s.is_empty());
    }
    Ok(cio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_by_position_and_defaults() {
        let text = "file.cio: test\n\
                    simulation  time.sim  print.prt  null  object.cnt  null\n\
                    anything    codes.bsn parameters.bsn\n";
        let cio = read(text).unwrap();
        assert_eq!(cio.title, "file.cio:");
        assert_eq!(cio.file("simulation", "time"), Some("time.sim"));
        assert_eq!(cio.file("simulation", "object_prt"), None);
        // The second row's name is ignored, and it is short: it goes on into the next line,
        // which is empty, so the read ends at the end of the file and the default stays.
        assert_eq!(cio.file("basin", "codes_bas"), Some("codes.bsn"));
        assert_eq!(cio.file("basin", "carbon_bsn"), Some("carbon.bsn"));
        // Rows past the end of the file keep their defaults.
        assert_eq!(cio.file("connect", "hru_con"), Some("hru.con"));
        assert_eq!(cio.file("pcp_path", "pcp"), None);
    }

    #[test]
    fn extra_names_are_ignored() {
        let mut text = String::from("t\n");
        for (key, slots) in SCHEMA {
            text.push_str(key);
            for i in 0..slots.len() + 2 {
                text.push_str(&format!(" f{i}"));
            }
            text.push('\n');
        }
        text.push_str("out_path  results/run1\n");
        let cio = read(&text).unwrap();
        assert_eq!(cio.file("init", "salt_water"), Some("f10"));
        assert_eq!(cio.file("wnd_path", "wnd"), Some("f0"));
        assert_eq!(cio.out_path.as_deref(), Some("results/run1"));
    }
}
