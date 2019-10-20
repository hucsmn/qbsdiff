mod common;

use common::*;
use globwalk::glob;
use std::fs;
use std::io;
use std::path;

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

struct Sample {
    name: String,
    source: path::PathBuf,
    target: path::PathBuf,
}

fn list_samples() -> io::Result<Vec<Sample>> {
    let data_dir = tests_dir().join("samples");

    let mut samples = Vec::new();
    let pat = data_dir.join("*.s");
    let walker;
    if let Some(p) = pat.to_str() {
        walker = glob(p).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "cannot convert to str",
        ));
    }
    for result in walker.into_iter() {
        let source = result
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .into_path();

        let name;
        let target;
        if let (Some(d), Some(n)) = (source.parent(), source.file_stem()) {
            let mut nbuf = n.to_owned();
            name = nbuf.to_string_lossy().into_owned();
            nbuf.push(".t");
            target = path::PathBuf::from(d).join(nbuf.as_os_str());
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "cannot make target path",
            ));
        }
        if let Err(_) = fs::metadata(target.as_path()) {
            continue;
        }

        samples.push(Sample {
            name,
            source,
            target,
        });
    }

    Ok(samples)
}
