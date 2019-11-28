use std::fs;
use std::path;
use utils::*;

#[test]
fn sample_compat() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_samples().unwrap();

    for sample in samples.iter() {
        eprintln!("compatibility test on sample `{}`", sample.name);
        let s = fs::read(sample.source.as_path()).unwrap();
        let t = fs::read(sample.target.as_path()).unwrap();

        let p1 = testing.bsdiff(&s[..], &t[..]).unwrap();
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

#[test]
fn random_compat() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let descs = testing.default_random_samples();
    let samples = testing.get_random_samples(descs.as_ref()).unwrap();

    for sample in samples.iter() {
        eprintln!("compatibility test on sample `{}`", sample.name);
        let s = fs::read(sample.source.as_path()).unwrap();
        let t = fs::read(sample.target.as_path()).unwrap();

        let p1 = testing.bsdiff(&s[..], &t[..]).unwrap();
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
