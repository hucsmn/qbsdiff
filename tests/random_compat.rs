mod common;

use common::*;
use rand::distributions::uniform::{SampleUniform, Uniform};
use rand::prelude::*;
use std::fs;
use std::io;
use std::path;

#[test]
fn random_qbsdiff_bspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        let s = fetch_file(sample.source.as_path()).unwrap();
        for (i, target) in sample.targets.iter().enumerate() {
            eprintln!("qbsdiff/bspatch `{}`/`{}`", sample.name, i);
            let t = fetch_file(target.as_path()).unwrap();
            let p = qbsdiff(&s[..], &t[..]).unwrap();
            let t1 = bspatch(&s[..], &p[..]).unwrap();
            if t != t1 {
                panic!("incompatible: qbsdiff/bspatch `{}`/`{}`", sample.name, i);
            }
        }
    }
}

#[test]
fn random_bsdiff_qbspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        let s = fetch_file(sample.source.as_path()).unwrap();
        for (i, target) in sample.targets.iter().enumerate() {
            eprintln!("bsdiff/qbspatch `{}`/`{}`", sample.name, i);
            let t = fetch_file(target.as_path()).unwrap();
            let p = bsdiff(&s[..], &t[..]).unwrap();
            let t1 = qbspatch(&s[..], &p[..]).unwrap();
            if t != t1 {
                panic!("incompatible: bsdiff/qbspatch `{}`/`{}`", sample.name, i);
            }
        }
    }
}

#[test]
fn random_qbsdiff_qbspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        let s = fetch_file(sample.source.as_path()).unwrap();
        for (i, target) in sample.targets.iter().enumerate() {
            eprintln!("qbsdiff/qbspatch `{}`/`{}`", sample.name, i);
            let t = fetch_file(target.as_path()).unwrap();
            let p = qbsdiff(&s[..], &t[..]).unwrap();
            let t1 = qbspatch(&s[..], &p[..]).unwrap();
            if t != t1 {
                panic!("incompatible: qbsdiff/qbspatch `{}`/`{}`", sample.name, i);
            }
        }
    }
}

struct Sample {
    name: &'static str,
    source: path::PathBuf,
    targets: Vec<path::PathBuf>,
}

struct SampleDesc {
    name: &'static str,
    source: SourceDesc,
    targets: Vec<TargetDesc>,
}

enum SourceDesc {
    Bytes(&'static [u8]),
    Random(usize),
}

enum TargetDesc {
    Bytes(&'static [u8]),
    Distort(f64),
}

fn list_samples() -> io::Result<Vec<Sample>> {
    let descs = default_sample_descs();
    make_samples(descs.as_slice())
}

fn default_sample_descs() -> Vec<SampleDesc> {
    use SourceDesc::{Bytes as SBytes, Random};
    use TargetDesc::{Bytes as TBytes, Distort};
    vec![
        SampleDesc {
            name: "empty",
            source: SBytes(b""),
            targets: vec![TBytes(b""), TBytes(b"extra")],
        },
        SampleDesc {
            name: "small",
            source: SBytes(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."),
            targets: vec![
                TBytes(b""),
                TBytes(b"consectetur adip##cing elit, jed do eiusmod wir mussen wissen. wir werden wissen/ laboris nisi ut al&^%ip ex ea coikodo consequat. "),
                TBytes(b"the quick brown fox jumps over the lazy dog"),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        SampleDesc {
            name: "rand-4k",
            source: Random(4096),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        SampleDesc {
            name: "rand-256k",
            source: Random(256*1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        SampleDesc {
            name: "rand-1m",
            source: Random(1024*1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        SampleDesc {
            name: "rand-8m",
            source: Random(8*1024*1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
    ]
}

fn make_samples(descs: &[SampleDesc]) -> io::Result<Vec<Sample>> {
    let dir = tests_dir().join("random-caches");
    fs::create_dir_all(dir.as_path())?;

    let mut samples = Vec::with_capacity(descs.len());
    for desc in descs.iter() {
        let source = dir.join(format!("{}.s", desc.name));
        let source_bytes;
        if !exists_file(source.as_path()) {
            match desc.source {
                SourceDesc::Bytes(bytes) => {
                    source_bytes = Vec::from(bytes);
                }
                SourceDesc::Random(size) => {
                    source_bytes = random_bytes(size);
                }
            }
            store_file(source.as_path(), &source_bytes[..])?;
        } else {
            source_bytes = fetch_file(source.as_path())?;
        }

        let mut targets = Vec::with_capacity(desc.targets.len());
        for (i, tdesc) in desc.targets.iter().enumerate() {
            let target = dir.join(format!("{}.t{}", desc.name, i));
            if !exists_file(target.as_path()) {
                match tdesc {
                    TargetDesc::Bytes(bytes) => {
                        store_file(target.as_path(), bytes)?;
                    }
                    TargetDesc::Distort(similar) => {
                        let target_bytes = distort(&source_bytes[..], *similar);
                        store_file(target.as_path(), target_bytes)?;
                    }
                }
            }
            targets.push(target);
        }

        samples.push(Sample {
            name: desc.name,
            source,
            targets,
        });
    }

    Ok(samples)
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        bytes.push(rng.gen())
    }
    bytes
}

fn distort(source: &[u8], similar: f64) -> Vec<u8> {
    let similar = fraction(similar);
    let rate = convex_mapping(similar);

    let tsize = random_between(
        (source.len() as f64 * 0.75) as usize,
        (source.len() as f64 * 1.25) as usize,
    );
    let dmax = random_between(
        Ord::min(16, (source.len() as f64 * 0.02) as usize),
        Ord::max(32, (source.len() as f64 * 0.33) as usize),
    );
    let emax = random_between(0, (source.len() as f64 * 0.15 * (1.0 - similar)) as usize);

    let mut target = Vec::with_capacity(tsize);
    let mut rng = thread_rng();
    while target.len() < tsize {
        // delta
        let remain = tsize - target.len();
        let dsize = {
            let dhi = Ord::min(Ord::min(dmax, remain), source.len());
            let dlo = Ord::min(16, dhi);
            random_between(dlo, dhi)
        };
        let offset = random_between(0, source.len() - dsize);
        for &x in source[offset..offset + dsize].iter() {
            if random_decide(rate) {
                target.push(x);
            } else {
                target.push(rng.gen());
            }
        }

        // extra
        let remain = tsize - target.len();
        if !random_decide(rate) {
            let esize = random_between(0, Ord::min(emax, remain));
            for _ in 0..esize {
                target.push(rng.gen());
            }
        }
    }

    target
}

fn random_decide(rate: f64) -> bool {
    random_between(0.0, 1.0) <= fraction(rate)
}

fn random_between<X: SampleUniform>(lo: X, hi: X) -> X {
    let mut rng = thread_rng();
    Uniform::new_inclusive(lo, hi).sample(&mut rng)
}

fn fraction(x: f64) -> f64 {
    if x.is_nan() || x.is_sign_negative() {
        0.0
    } else if x.is_infinite() || x > 1.0 {
        1.0
    } else {
        x
    }
}

fn convex_mapping(frac: f64) -> f64 {
    (1.0 - (1.0 - frac) * (1.0 - frac)).sqrt()
}
