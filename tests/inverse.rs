mod common;

extern crate quickcheck;
extern crate quickcheck_macros;

use common::*;
use quickcheck_macros::*;
use std::io;

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
fn invert_random(s: Vec<u8>, t: Vec<u8>) -> io::Result<bool> {
    let p = qbsdiff(&s[..], &t[..])?;
    return Ok(qbspatch(&s[..], &p[..])? == t);
}
