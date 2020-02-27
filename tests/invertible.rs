use qbsdiff_test_bench_utils::*;
use std::path;

#[test]
fn regular_samples_invert() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_regular_samples().unwrap();

    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = testing.qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("not invertible: `{}`", sample.name);
        }
    }
}

#[test]
fn pathological_samples_invert() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_pathological_samples().unwrap();

    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = testing.qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("not invertible: `{}`", sample.name);
        }
    }
}

#[test]
fn random_samples_invert() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let descs = default_random_samples();
    let testing = Testing::new(assets);
    let samples = testing.get_random_samples(descs.as_ref()).unwrap();

    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();

        let p = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = testing.qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("not invertible: `{}`", sample.name);
        }
    }
}
