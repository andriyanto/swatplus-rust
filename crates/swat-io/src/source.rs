//! Where files come from: a folder on disk, or files held in memory.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::{Error, Result, decode};

/// A set of input files, addressed by the names (and relative paths) SWAT+ files use.
pub trait FileSource {
    /// The text of file `name`, `None` if there is no such file (SWAT+ `inquire (exist=…)`).
    fn read(&self, name: &str) -> Result<Option<String>>;
}

/// A scenario folder on disk (the TxtInOut folder).
pub struct DirSource {
    root: PathBuf,
}

impl DirSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl FileSource for DirSource {
    fn read(&self, name: &str) -> Result<Option<String>> {
        let path = self.root.join(name);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(decode(&bytes))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::Io { file: name.to_string(), source: e }),
        }
    }
}

/// Files held in memory, by name: a scenario unpacked in the browser, or built in tests.
#[derive(Default)]
pub struct MemSource {
    files: HashMap<String, String>,
}

impl MemSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, bytes: &[u8]) {
        self.files.insert(name.into(), decode(bytes));
    }
}

impl FileSource for MemSource {
    fn read(&self, name: &str) -> Result<Option<String>> {
        Ok(self.files.get(name).cloned())
    }
}
