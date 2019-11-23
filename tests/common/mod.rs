#![allow(unused)]

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

pub fn bsdiff(s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
    let bin = get_binary("bsdiff")?;

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
    Bsdiff::new(s, t).compare(io::Cursor::new(&mut p))?;
    Ok(p)
}

pub fn bspatch(s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
    let bin = get_binary("bspatch")?;

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

#[cfg(windows)]
fn get_binary(name: &'static str) -> io::Result<path::PathBuf> {
    Ok(tests_dir().join("bin").join(format!("{}.exe", name)))
}

#[cfg(unix)]
fn get_binary(name: &'static str) -> io::Result<path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let bin = tests_dir().join("bin").join(name);
    fs::set_permissions(bin.as_path(), fs::Permissions::from_mode(0o755))?;
    Ok(bin)
}

pub fn create_temp<B: AsRef<[u8]>>(bytes: B) -> io::Result<path::PathBuf> {
    let dir = std::env::temp_dir().join("qbsdiff-test");
    fs::create_dir_all(dir.as_path())?;

    let id = format!("{}-{:x}", Utc::now().format("%s.%f"), random::<u32>());
    let p = dir.join(id);

    store_file(p.as_path(), bytes)?;
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

pub fn store_file<P, B>(name: P, bytes: B) -> io::Result<()>
where
    P: AsRef<path::Path>,
    B: AsRef<[u8]>,
{
    let mut file = File::create(name.as_ref())?;
    file.write_all(bytes.as_ref())
}

pub fn exists_file<P: AsRef<path::Path>>(name: P) -> bool {
    if let Ok(meta) = fs::metadata(name) {
        meta.is_file()
    } else {
        false
    }
}

pub struct Sample {
    pub name: String,
    pub source: path::PathBuf,
    pub target: path::PathBuf,
}

pub fn list_samples() -> io::Result<Vec<Sample>> {
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
