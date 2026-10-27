//! Readers for SWAT+ TxtInOut files.
//!
//! Each reader follows the Fortran routine that reads the file in SWAT+ (named in its docs), with
//! Fortran's list-directed input rules ([`list`]), so a file means here what it means to SWAT+:
//! the same values, the same defaults, the same leniency. Files come from a [`FileSource`], a
//! folder on disk or files held in memory (the browser), so nothing here touches the disk itself.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable))]

pub mod basin;
pub mod cal;
pub mod cio;
pub mod climate;
pub mod con;
pub mod hru;
pub mod list;
pub mod print;
pub mod project;
pub mod recall;
pub mod routing;
pub mod source;
pub mod table;
pub mod time;

pub use project::{LoadReport, load_project};
pub use source::{DirSource, FileSource, MemSource};

/// Errors from reading SWAT+ files.
#[derive(Debug)]
pub enum Error {
    Parse { file: String, line: usize, msg: String },
    Eof { file: String },
    Io { file: String, source: std::io::Error },
    Missing(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse { file, line, msg } => write!(f, "{file}: line {line}: {msg}"),
            Error::Eof { file } => write!(f, "{file}: unexpected end of file"),
            Error::Io { file, source } => write!(f, "{file}: {source}"),
            Error::Missing(file) => write!(f, "{file}: not found"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Text of a file's bytes. SWAT+ files are ASCII, but some carry Latin-1 bytes (names written by
/// older tools): each byte becomes the character with that code, so nothing fails to decode.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}
