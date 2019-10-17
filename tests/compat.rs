use chrono::Utc;
use globwalk::glob;
use qbsdiff::{Bsdiff, Bspatch};
use rand::random;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path;
use subprocess::Exec;

#[test]
fn qbsdiff_bspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("qbsdiff/bspatch `{}`: s <- `{:?}`", sample.name, sample.source);
        let s = fetch_file(sample.source.as_path()).unwrap();

        eprintln!("qbsdiff/bspatch `{}`: t <- `{:?}`", sample.name, sample.target);
        let t = fetch_file(sample.target.as_path()).unwrap();

        eprintln!("qbsdiff/bspatch `{}`: p <- qbsdiff(s, t)", sample.name);
        let p = qbsdiff(&s[..], &t[..]).unwrap();

        eprintln!("qbsdiff/bspatch `{}`: bspatch(s, p) ?= t", sample.name);
        let t1 = bspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("[{}] bspatch(s, qbsdiff(s, t)) != t", sample.name);
        }
    }
}

#[test]
fn bsdiff_qbspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("bsdiff/qbspatch `{}`: s <- `{:?}`", sample.name, sample.source);
        let s = fetch_file(sample.source.as_path()).unwrap();

        eprintln!("bsdiff/qbspatch `{}`: t <- `{:?}`", sample.name, sample.target);
        let t = fetch_file(sample.target.as_path()).unwrap();

        eprintln!("bsdiff/qbspatch `{}`: p <- bsdiff(s, t)", sample.name);
        let p = bsdiff(&s[..], &t[..]).unwrap();

        eprintln!("bsdiff/qbspatch `{}`: qbspatch(s, p) ?= t", sample.name);
        let t1 = qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("[{}] qbspatch(s, bsdiff(s, t)) != t", sample.name);
        }
    }
}

#[test]
fn qbsdiff_qbspatch_compat() {
    let samples = list_samples().unwrap();
    for sample in samples.iter() {
        eprintln!("qbsdiff/qbspatch `{}`: s <- `{:?}`", sample.name, sample.source);
        let s = fetch_file(sample.source.as_path()).unwrap();

        eprintln!("qbsdiff/qbspatch `{}`: t <- `{:?}`", sample.name, sample.target);
        let t = fetch_file(sample.target.as_path()).unwrap();

        eprintln!("qbsdiff/qbspatch `{}`: p <- qbsdiff(s, t)", sample.name);
        let p = qbsdiff(&s[..], &t[..]).unwrap();

        eprintln!("qbsdiff/qbspatch `{}`: qbspatch(s, p) ?= t", sample.name);
        let t1 = qbspatch(&s[..], &p[..]).unwrap();
        if t != t1 {
            panic!("[{}] qbspatch(s, qbsdiff(s, t)) != t", sample.name);
        }
    }
}

// wrappers

fn bsdiff(s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
    let mut bin = tests_dir().join("bin");
    if cfg!(windows) {
        bin = bin.join("bsdiff.exe");
    } else {
        bin = bin.join("bsdiff");
    }

    let spath = create_temp(s)?;
    let tpath = create_temp(t)?;
    let ppath = create_temp(b"")?;
    let succ = Exec::cmd(bin)
        .args(&[spath.as_os_str(), tpath.as_os_str(), ppath.as_os_str()])
        .capture()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        .exit_status
        .success();
    if !succ {
        return Err(io::Error::new(io::ErrorKind::Other, "execution failed"));
    }

    fetch_file(ppath)
}

fn qbsdiff(s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
    let mut p = Vec::new();
    Bsdiff::new(s).compare(t, io::Cursor::new(&mut p))?;
    Ok(p)
}

fn bspatch(s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
    let mut bin = tests_dir().join("bin");
    if cfg!(windows) {
        bin = bin.join("bspatch.exe");
    } else {
        bin = bin.join("bspatch");
    }

    let spath = create_temp(s)?;
    let tpath = create_temp(b"")?;
    let ppath = create_temp(p)?;
    let succ = Exec::cmd(bin)
        .args(&[spath.as_os_str(), tpath.as_os_str(), ppath.as_os_str()])
        .capture()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        .exit_status
        .success();
    if !succ {
        return Err(io::Error::new(io::ErrorKind::Other, "execution failed"));
    }

    fetch_file(tpath)
}

fn qbspatch(s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
    let patcher = Bspatch::new(p)?;
    let mut t = Vec::with_capacity(patcher.hint_target_size() as usize);
    patcher.apply(s, io::Cursor::new(&mut t))?;
    Ok(t)
}

// utilities

struct Sample {
    pub name: String,
    pub source: path::PathBuf,
    pub target: path::PathBuf,
}

fn tests_dir() -> path::PathBuf {
    path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn list_samples() -> io::Result<Vec<Sample>> {
    let data_dir = tests_dir().join("compat-data");

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

fn create_temp<B: AsRef<[u8]>>(bytes: B) -> io::Result<path::PathBuf> {
    let dir = std::env::temp_dir().join("qbsdiff-test");
    fs::create_dir_all(dir.as_path())?;

    let id = format!("{}-{:x}", Utc::now().format("%s.%f"), random::<u32>());
    let p = dir.join(id);

    let mut f = File::create(p.as_path())?;
    f.write_all(bytes.as_ref())?;
    Ok(p)
}

fn fetch_file<P: AsRef<path::Path>>(name: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(name)?;
    let size = file.seek(io::SeekFrom::End(0))?;
    if size > std::usize::MAX as u64 {
        return Err(io::Error::new(io::ErrorKind::Other, "file too large"));
    }

    let mut data = Vec::with_capacity(size as usize);
    file.seek(io::SeekFrom::Start(0))?;
    file.read_to_end(&mut data)?;
    Ok(data)
}
