use std::path;
use qbsdiff_test_bench_utils::*;

#[test]
fn regular_samples_compat() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_regular_samples().unwrap();

    for sample in samples.iter() {
        eprintln!("compatibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p1 = testing.load_cached_patch(&sample).unwrap();
        let t1 = testing.qbspatch(&s[..], &p1[..]).unwrap();
        if t1 != t {
            panic!("bsdiff/qbspatch incompatible: `{}`", sample.name);
        }

        let p2 = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t2 = testing.bspatch(&s[..], &p2[..]).unwrap();
        if t2 != t {
            panic!("qbsdiff/bspatch incompatible: `{}`", sample.name);
        }
    }
}

// Original bsdiff(1) runs extremely slow on some pathological samples.
// Therefore, we simply do not test compatibility on those samples here.

#[test]
fn random_samples_compat() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let descs = default_random_samples();
    let testing = Testing::new(assets);
    let samples = testing.get_random_samples(descs.as_ref()).unwrap();

    for sample in samples.iter() {
        eprintln!("compatibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p1 = testing.load_cached_patch(&sample).unwrap();
        let t1 = testing.qbspatch(&s[..], &p1[..]).unwrap();
        if t1 != t {
            panic!("bsdiff/qbspatch incompatible: `{}`", sample.name);
        }

        let p2 = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t2 = testing.bspatch(&s[..], &p2[..]).unwrap();
        if t2 != t {
            panic!("qbsdiff/bspatch incompatible: `{}`", sample.name);
        }
    }
}
