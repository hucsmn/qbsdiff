mod common;

use common::*;

extern crate quickcheck;
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[test]
fn invert_samples() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("qbsdiff/bspatch `{}`", sample.name);
        let s = fetch_file(sample.source.as_path()).unwrap();
        let t = fetch_file(sample.target.as_path()).unwrap();
        let p = qbsdiff(&s[..], &t[..]).unwrap();
        let t1 = qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("incompatible: qbsdiff/bspatch `{}`", sample.name);
        }
    }
}

#[quickcheck]
fn invert_random(s: Vec<u8>, t: Vec<u8>) -> bool {
    let p = qbsdiff(&s[..], &t[..]).unwrap();
    return qbspatch(&s[..], &p[..]).unwrap() == t;
}
