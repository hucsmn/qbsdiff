use chrono::Utc;
use qbsdiff::{Bsdiff, Bspatch};
use rand::random;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path;
use subprocess::Exec;

pub fn bsdiff(s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
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

pub fn qbsdiff(s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
    let mut p = Vec::new();
    Bsdiff::new(s).compare(t, io::Cursor::new(&mut p))?;
    Ok(p)
}

pub fn bspatch(s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
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

pub fn qbspatch(s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
    let patcher = Bspatch::new(p)?;
    let mut t = Vec::with_capacity(patcher.hint_target_size() as usize);
    patcher.apply(s, io::Cursor::new(&mut t))?;
    Ok(t)
}

pub fn tests_dir() -> path::PathBuf {
    path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

pub fn create_temp<B: AsRef<[u8]>>(bytes: B) -> io::Result<path::PathBuf> {
    let dir = std::env::temp_dir().join("qbsdiff-test");
    fs::create_dir_all(dir.as_path())?;

    let id = format!("{}-{:x}", Utc::now().format("%s.%f"), random::<u32>());
    let p = dir.join(id);

    let mut f = File::create(p.as_path())?;
    f.write_all(bytes.as_ref())?;
    Ok(p)
}

pub fn fetch_file<P: AsRef<path::Path>>(name: P) -> io::Result<Vec<u8>> {
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
