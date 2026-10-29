//! SWAT+ in Rust as a WebAssembly module, for the browser (RustGIS runs it in a Web Worker).
//! It takes no dependencies and needs no JavaScript glue:
//!
//! - `swat_alloc(len) -> ptr` and `swat_free(ptr, len)` manage the module's memory;
//! - `swat_run(ptr, len, flags) -> ptr` runs the scenario in the tar archive at `ptr` (its
//!   TxtInOut files, in a folder or not) and returns a buffer: 4 bytes of little-endian length,
//!   then a tar archive of the output tables and `run.json` (the notes, or the error). Flags:
//!   1 Parquet tables, 2 text tables (both when neither is set), 4 correct SWAT+'s bugs
//!   (`--fix-swat`). The caller frees the buffer with `swat_free(ptr, 4 + length)`;
//! - the module imports `env.swat_progress(done, days)`, called at each whole percent of the
//!   simulated days.
//!
//! [`run_tar`] does the same from Rust.

use swat_io::{MemSource, load_project};

/// What to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub parquet: bool,
    pub text: bool,
    pub fixes: bool,
}

impl Options {
    pub fn from_flags(flags: u32) -> Self {
        let (p, t) = (flags & 1 != 0, flags & 2 != 0);
        Options { parquet: p, text: t || !p, fixes: flags & 4 != 0 }
    }
}

/// Runs the scenario in tar archive `input`; returns a tar archive of the tables and
/// `run.json`. `progress` gets (days done, days) at each whole percent.
pub fn run_tar(input: &[u8], opts: Options, progress: Box<dyn FnMut(usize, usize)>) -> Vec<u8> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let error = run_inner(input, opts, progress, &mut files, &mut notes).err();
    let q = swat_output::progress::quote;
    let notes_json: Vec<String> = notes.iter().map(|n| q(n)).collect();
    let tables: Vec<String> = files.iter().map(|(n, _)| q(n)).collect();
    let report = format!(
        "{{\"version\":{},\"ok\":{},\"error\":{},\"notes\":[{}],\"tables\":[{}]}}\n",
        q(env!("CARGO_PKG_VERSION")),
        error.is_none(),
        error.as_deref().map_or("null".to_string(), q),
        notes_json.join(","),
        tables.join(",")
    );
    files.push(("run.json".to_string(), report.into_bytes()));
    swat_output::tar::write(files.iter().map(|(n, b)| (n.as_str(), b.as_slice())))
}

fn run_inner(
    input: &[u8],
    opts: Options,
    mut progress: Box<dyn FnMut(usize, usize)>,
    files: &mut Vec<(String, Vec<u8>)>,
    notes: &mut Vec<String>,
) -> Result<(), String> {
    let mut src = MemSource::new();
    for (name, bytes) in swat_output::tar::read(input)? {
        src.insert(name, &bytes);
    }
    let (project, _) = load_project(&src).map_err(|e| e.to_string())?;
    let title = format!(" {:<24} swatplus-rust {}", project.cio.title.trim_end_matches(':'), env!("CARGO_PKG_VERSION"));
    let mut tables = swat_output::TextTables::new(&title, swat_output::MemStore::new(), &project.print);
    tables.fixes = opts.fixes;
    tables.progress = Some(swat_output::progress::Reporter::new(Box::new(move |line: &str| {
        // The reporter's JSON line, read back for the two numbers.
        let num = |key: &str| -> usize {
            line.split(&format!("\"{key}\":"))
                .nth(1)
                .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };
        progress(num("done"), num("days"));
    })));
    let report = swat_engine::run::run_with(&project, &mut tables, swat_engine::run::RunOptions { fixes: opts.fixes, threads: 1 })?;
    if let Some(why) = report.hru_skipped {
        notes.push(format!("HRU water balance not simulated ({why})"));
    }
    if let Some(why) = report.channel_skipped {
        notes.push(format!("channels not routed ({why})"));
    }
    if let Some(why) = report.routing_skipped {
        notes.push(format!("routing units and aquifers not simulated ({why})"));
    }
    let store = tables.finish()?;
    for (name, text) in store.files {
        if opts.parquet {
            let stem = name.strip_suffix(".txt").unwrap_or(&name).to_string();
            let (_, pq) =
                swat_output::parquet::text_to_parquet(text.as_slice(), Vec::new(), 1 << 20).map_err(|e| format!("{stem}.parquet: {e}"))?;
            files.push((format!("{stem}.parquet"), pq));
        }
        if opts.text {
            files.push((name, text));
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
mod ffi {
    use super::{Options, run_tar};

    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn swat_progress(done: u32, days: u32);
    }

    /// A buffer of `len` bytes for the caller to fill.
    #[unsafe(no_mangle)]
    pub extern "C" fn swat_alloc(len: usize) -> *mut u8 {
        let mut v = vec![0u8; len];
        let p = v.as_mut_ptr();
        std::mem::forget(v);
        p
    }

    /// Frees a buffer from `swat_alloc` or `swat_run`.
    ///
    /// # Safety
    /// `ptr` and `len` must be a buffer this module gave out, not freed before.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn swat_free(ptr: *mut u8, len: usize) {
        if !ptr.is_null() {
            drop(unsafe { Vec::from_raw_parts(ptr, len, len) });
        }
    }

    /// Runs the scenario in the tar archive at `ptr`; see the crate's documentation.
    ///
    /// # Safety
    /// `ptr` must hold `len` bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn swat_run(ptr: *const u8, len: usize, flags: u32) -> *mut u8 {
        let input = unsafe { std::slice::from_raw_parts(ptr, len) };
        let out = run_tar(input, Options::from_flags(flags), Box::new(|d, n| unsafe { swat_progress(d as u32, n as u32) }));
        let mut buf = Vec::with_capacity(4 + out.len());
        buf.extend_from_slice(&(out.len() as u32).to_le_bytes());
        buf.extend_from_slice(&out);
        let mut buf = buf.into_boxed_slice().into_vec();
        buf.shrink_to_fit();
        let p = buf.as_mut_ptr();
        std::mem::forget(buf);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osu_through_tar() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/Osu_1hru");
        let mut files = Vec::new();
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.path().is_file() {
                files.push((format!("TxtInOut/{}", e.file_name().to_string_lossy()), std::fs::read(e.path()).unwrap_or_default()));
            }
        }
        let input = swat_output::tar::write(files.iter().map(|(n, b)| (n.as_str(), b.as_slice())));
        let calls = std::rc::Rc::new(std::cell::Cell::new(0usize));
        let c = calls.clone();
        let out = run_tar(&input, Options::from_flags(3), Box::new(move |_, _| c.set(c.get() + 1)));
        let tables = swat_output::tar::read(&out).unwrap_or_default();
        let report = tables.iter().find(|(n, _)| n == "run.json").map(|(_, b)| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
        assert!(report.contains("\"ok\":true"), "{report}");
        assert!(tables.iter().any(|(n, _)| n == "hru_wb_aa.parquet"));
        assert!(tables.iter().any(|(n, _)| n == "hru_wb_aa.txt"));
        assert!(calls.get() > 50);
    }
}
