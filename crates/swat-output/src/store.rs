//! Where the output files go: a folder on disk, or memory (the browser).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// A place to write files to, a piece at a time.
pub trait Store {
    /// Appends `bytes` to file `name`, creating it on the first call.
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>;
}

/// Files in a folder.
pub struct DirStore {
    dir: PathBuf,
    files: BTreeMap<String, File>,
}

impl DirStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into(), files: BTreeMap::new() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The names of the files written, in order.
    pub fn names(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }
}

impl Store for DirStore {
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.dir.join(name);
        if !self.files.contains_key(name) {
            let f = File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            self.files.insert(name.to_string(), f);
        }
        match self.files.get_mut(name) {
            Some(f) => f.write_all(bytes).map_err(|e| format!("{}: {e}", path.display())),
            None => Err(format!("{}: not open", path.display())),
        }
    }
}

/// Files in memory, by name.
#[derive(Default)]
pub struct MemStore {
    pub files: BTreeMap<String, Vec<u8>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for MemStore {
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.files.entry(name.to_string()).or_default().extend_from_slice(bytes);
        Ok(())
    }
}
