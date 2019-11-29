use std::path;
use utils::*;

#[test]
fn sample_invert() {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let testing = Testing::new(assets);
    let samples = testing.get_regular_samples().unwrap();

    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let (s, t) = sample.load().unwrap();
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
    let descs = default_random_samples();
    let testing = Testing::new(assets);
    let samples = testing.get_random_samples(descs.as_ref()).unwrap();

    for sample in samples.iter() {
        eprintln!("invertibility test on sample `{}`", sample.name);
        let (s, t) = sample.load().unwrap();
        let p = testing.qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = testing.qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("not invertible: `{}`", sample.name);
        }
    }
}
