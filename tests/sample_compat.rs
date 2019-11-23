mod common;

use common::*;

#[test]
fn qbsdiff_bspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("qbsdiff/bspatch `{}`", sample.name);
        let s = fetch_file(sample.source.as_path()).unwrap();
        let t = fetch_file(sample.target.as_path()).unwrap();
        let p = qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = bspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("incompatible: qbsdiff/bspatch `{}`", sample.name);
        }
    }
}

#[test]
fn bsdiff_qbspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("bsdiff/qbspatch `{}`", sample.name);
        let s = fetch_file(sample.source.as_path()).unwrap();
        let t = fetch_file(sample.target.as_path()).unwrap();
        let p = bsdiff(&s[..], &t[..]).unwrap();
        let t1 = qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("incompatible: bsdiff/qbspatch `{}`", sample.name);
        }
    }
}

#[test]
fn qbsdiff_qbspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("qbsdiff/qbspatch `{}`", sample.name);
        let s = fetch_file(sample.source.as_path()).unwrap();
        let t = fetch_file(sample.target.as_path()).unwrap();
        let p = qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("incompatible: qbsdiff/qbspatch `{}`", sample.name);
        }
    }
}
