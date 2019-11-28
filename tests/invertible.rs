use std::fs;
use std::path;
use utils::*;

#[test]
fn sample_invert() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let s = fs::read(sample.source.as_path()).unwrap();
        let t = fs::read(sample.target.as_path()).unwrap();
        let p = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = testing.qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("not invertible: `{}`", sample.name);
        }
    }
}

#[test]
fn random_invert() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let descs = testing.default_random_samples();
    let samples = testing.get_random_samples(descs.as_ref()).unwrap();
    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let s = fs::read(sample.source.as_path()).unwrap();
        let t = fs::read(sample.target.as_path()).unwrap();
        let p = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = testing.qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("not invertible: `{}`", sample.name);
        }
    }
}
