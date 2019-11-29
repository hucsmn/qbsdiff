use chrono::Utc;
use qbsdiff::{Bsdiff, Bspatch};
use rand::random;
use std::fs;
use std::io;
use std::path;
use globwalk::glob;
use subprocess::Exec;
use rand::distributions::uniform::{SampleUniform, Uniform};
use rand::prelude::*;

/// The testing context.
pub struct Testing {
    assets_dir: path::PathBuf,
}

impl Testing {
    /// Create new tesing context.
    pub fn new(assets_dir: path::PathBuf) -> Self {
        Testing { assets_dir }
    }

    /// Execute bsdiff command.
    pub fn bsdiff(&self, s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
        let dir = self.assets_dir.join("bin");
        run_bsdiff_in(dir, s, t)
    }

    /// Execute bspatch command.
    pub fn bspatch(&self, s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
        let dir = self.assets_dir.join("bin");
        run_bspatch_in(dir, s, p)
    }

    /// Perform qbsdiff.
    pub fn qbsdiff(&self, s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
        let mut p = Vec::new();
        Bsdiff::new(s, t).compare(io::Cursor::new(&mut p))?;
        Ok(p)
    }

    /// Perform qbspatch.
    pub fn qbspatch(&self, s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
        let patcher = Bspatch::new(p)?;
        let mut t = Vec::with_capacity(patcher.hint_target_size() as usize);
        patcher.apply(s, io::Cursor::new(&mut t))?;
        Ok(t)
    }

    /// Get regular samples.
    pub fn get_regular_samples(&self) -> io::Result<Vec<Sample>> {
        let dir = self.assets_dir.join("samples");
        get_samples_in(dir)
    }

    /// Get pathological samples.
    pub fn get_pathological_samples(&self) -> io::Result<Vec<Sample>> {
        let dir = self.assets_dir.join("pathological");
        get_samples_in(dir)
    }

    /// Prepare random samples if needed and get the sample list.
    pub fn get_random_samples(&self, descs: &[RandomSample]) -> io::Result<Vec<Sample>> {
        let dir = self.assets_dir.join("caches").join("random-samples");
        get_random_caches_in(dir, descs)
    }
}

fn run_bsdiff_in<P: AsRef<path::Path>>(dir: P, s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
    let bin = get_binary_in(dir, "bsdiff")?;

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
        return Err(io::Error::new(io::ErrorKind::Other, "bsdiff execution failed"));
    }

    fs::read(ppath)
}

fn run_bspatch_in<P: AsRef<path::Path>>(dir: P, s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
    let bin = get_binary_in(dir, "bspatch")?;

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
        return Err(io::Error::new(io::ErrorKind::Other, "bspatch execution failed"));
    }

    fs::read(tpath)
}

#[cfg(windows)]
fn get_binary_in<P: AsRef<path::Path>>(dir: P, name: &'static str) -> io::Result<path::PathBuf> {
    Ok(dir.as_ref().join(format!("{}.exe", name)))
}

#[cfg(unix)]
fn get_binary_in<P: AsRef<path::Path>>(dir: P, name: &'static str) -> io::Result<path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.as_ref().join(name);
    fs::set_permissions(bin.as_path(), fs::Permissions::from_mode(0o755))?;
    Ok(bin)
}

/// Test sample.
pub struct Sample {
    pub name: String,
    pub source: path::PathBuf,
    pub target: path::PathBuf,
}

impl Sample {
    /// Load sample data.
    pub fn load(&self) -> io::Result<(Vec<u8>, Vec<u8>)> {
        let s = fs::read(self.source.as_path())?;
        let t = fs::read(self.target.as_path())?;
        Ok((s, t))
    }
}

fn get_samples_in<P: AsRef<path::Path>>(dir: P) -> io::Result<Vec<Sample>> {
    let mut samples = Vec::new();
    let pat = dir.as_ref().join("*.s");
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

fn get_random_caches_in<P: AsRef<path::Path>>(dir: P, descs: &[RandomSample]) -> io::Result<Vec<Sample>> {
    fs::create_dir_all(dir.as_ref())?;

    let mut samples = Vec::new();
    for desc in descs.iter() {
        let source = dir.as_ref().join(format!("{}.s", desc.name));
        let source_bytes;
        if !exists_file(source.as_path()) {
            match desc.source {
                RandomSource::Bytes(bytes) => {
                    source_bytes = Vec::from(bytes);
                }
                RandomSource::Random(size) => {
                    source_bytes = random_bytes(size);
                }
            }
            fs::write(source.as_path(), &source_bytes[..])?;
        } else {
            source_bytes = fs::read(source.as_path())?;
        }

        for (i, tdesc) in desc.targets.iter().enumerate() {
            let target = dir.as_ref().join(format!("{}.t{}", desc.name, i));
            if !exists_file(target.as_path()) {
                match tdesc {
                    RandomTarget::Bytes(bytes) => {
                        fs::write(target.as_path(), bytes)?;
                    }
                    RandomTarget::Distort(similar) => {
                        let target_bytes = distort(&source_bytes[..], *similar);
                        fs::write(target.as_path(), target_bytes)?;
                    }
                }
            }
            samples.push(Sample {
                name: format!("{}/{}", desc.name, i),
                source: source.clone(),
                target,
            });
        }
    }

    Ok(samples)
}

/// Description of the random sample.
pub struct RandomSample {
    pub name: &'static str,
    pub source: RandomSource,
    pub targets: Vec<RandomTarget>,
}

/// Description of the source of random sample.
pub enum RandomSource {
    Bytes(&'static [u8]),
    Random(usize),
}

/// Description of a target of the random sample.
pub enum RandomTarget {
    Bytes(&'static [u8]),
    Distort(f64),
}

/// Default random sample descriptions.
pub fn default_random_samples() -> Vec<RandomSample> {
    use RandomSource::{Bytes as SBytes, Random};
    use RandomTarget::{Bytes as TBytes, Distort};

    vec![
        RandomSample {
            name: "empty",
            source: SBytes(b""),
            targets: vec![TBytes(b""), TBytes(b"extra")],
        },
        RandomSample {
            name: "small",
            source: SBytes(
b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempo\
r incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis no\
strud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Dui\
s aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fu\
giat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in cu\
lpa qui officia deserunt mollit anim id est laborum."
            ),
            targets: vec![
                TBytes(b""),
                TBytes(
b"consectetur adip##cing elit, jed do eiusmod wir mussen wissen. wir werden wis\
sen/ laboris nisi ut al&^%ip ex ea coikodo consequat. "
                ),
                TBytes(b"the quick brown fox jumps over the lazy dog"),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        RandomSample {
            name: "rand-4k",
            source: Random(4096),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        RandomSample {
            name: "rand-256k",
            source: Random(256*1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        RandomSample {
            name: "rand-1m",
            source: Random(1024*1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        RandomSample {
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

fn create_temp<B: AsRef<[u8]>>(bytes: B) -> io::Result<path::PathBuf> {
    let dir = std::env::temp_dir().join("qbsdiff-test");
    fs::create_dir_all(dir.as_path())?;

    let id = format!("{}-{:x}", Utc::now().format("%s.%f"), random::<u32>());
    let p = dir.join(id);

    fs::write(p.as_path(), bytes)?;
    Ok(p)
}

fn exists_file<P: AsRef<path::Path>>(name: P) -> bool {
    if let Ok(meta) = fs::metadata(name) {
        meta.is_file()
    } else {
        false
    }
}
