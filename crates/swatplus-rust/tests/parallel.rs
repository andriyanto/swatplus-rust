//! F1b (`docs/parallel.md`): with `--fix-swat` the HRUs run on any number of threads with the
//! same results, and without it `threads` changes nothing.

use std::path::PathBuf;

use swat_engine::run::{RunOptions, run_with};
use swat_io::{DirSource, load_project};
use swat_output::{MemStore, TextTables};

fn tables(name: &str, fixes: bool, threads: usize) -> std::collections::BTreeMap<String, Vec<u8>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata").join(name);
    let p = load_project(&DirSource::new(dir)).unwrap().0;
    let mut t = TextTables::new("parallel", MemStore::new(), &p.print);
    t.fixes = fixes;
    run_with(&p, &mut t, RunOptions { fixes, threads }).unwrap();
    t.finish().unwrap().files
}

#[test]
fn any_number_of_threads_gives_the_same_tables() {
    let one = tables("Ames_sub1", true, 1);
    for n in [3] {
        let many = tables("Ames_sub1", true, n);
        assert_eq!(one.keys().collect::<Vec<_>>(), many.keys().collect::<Vec<_>>());
        for (k, v) in &one {
            assert!(many[k] == *v, "{k} differs with {n} threads");
        }
    }
}

#[test]
fn threads_change_nothing_in_a_faithful_run() {
    assert!(tables("Osu_1hru", false, 1) == tables("Osu_1hru", false, 4));
}
