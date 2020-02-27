use std::path;
use qbsdiff_test_bench_utils::*;

// Parallel chunk size to test.
const CHUNK_SIZE: usize = 4096;

#[test]
fn regular_samples_par_compat() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_regular_samples().unwrap();
    let opts = QbsdiffOptions {
        chunk_size: CHUNK_SIZE,
        .. QbsdiffOptions::default()
    };

    for sample in samples.iter() {
        eprintln!("parallel compatibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p = testing.qbsdiff_with(&s[..], &t[..], opts).unwrap();
        let t1 = testing.bspatch(&s[..], &p[..]).unwrap();
        if t1 != t {
            panic!("par-qbsdiff/bspatch incompatible: `{}`", sample.name);
        }
    }
}

// Original bsdiff(1) runs extremely slow on some pathological samples.
// Therefore, we simply do not test compatibility on those samples here.

#[test]
fn random_samples_par_compat() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let descs = default_random_samples();
    let testing = Testing::new(assets);
    let samples = testing.get_random_samples(descs.as_ref()).unwrap();
    let opts = QbsdiffOptions {
        chunk_size: CHUNK_SIZE,
        .. QbsdiffOptions::default()
    };

    for sample in samples.iter() {
        eprintln!("parallel compatibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p = testing.qbsdiff_with(&s[..], &t[..], opts).unwrap();
        let t1 = testing.bspatch(&s[..], &p[..]).unwrap();
        if t1 != t {
            panic!("par-qbsdiff/bspatch incompatible: `{}`", sample.name);
        }
    }
}
