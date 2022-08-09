use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path;
use std::path::Path;
use std::process;

use chrono::Utc;
use globwalk::glob;
use rand::distributions::uniform::{SampleUniform, Uniform};
use rand::prelude::*;
use rand::random;

use qbsdiff::{Bsdiff, Bspatch, ParallelScheme};

/// Options for qbsdiff.
#[derive(Copy, Clone, Debug)]
pub struct QbsdiffOptions {
    pub chunk_size: usize,
    pub small_match: usize,
    pub compression_level: u32,
    pub buffer_size: usize,
}

impl Default for QbsdiffOptions {
    fn default() -> Self {
        QbsdiffOptions {
            chunk_size: 0,
            small_match: qbsdiff::bsdiff::SMALL_MATCH,
            compression_level: qbsdiff::bsdiff::COMPRESSION_LEVEL,
            buffer_size: qbsdiff::bsdiff::BUFFER_SIZE,
        }
    }
}

/// Options for qbspatch.
#[derive(Copy, Clone, Debug)]
pub struct QbspatchOptions {
    pub buffer_size: usize,
    pub delta_min: usize,
}

impl Default for QbspatchOptions {
    fn default() -> Self {
        QbspatchOptions {
            buffer_size: qbsdiff::bspatch::BUFFER_SIZE,
            delta_min: qbsdiff::bspatch::DELTA_MIN,
        }
    }
}

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

    /// Perform qbsdiff with options.
    pub fn qbsdiff_with(&self, s: &[u8], t: &[u8], opts: QbsdiffOptions) -> io::Result<Vec<u8>> {
        let mut p = Vec::new();
        Bsdiff::new(s, t)
            .parallel_scheme(ParallelScheme::ChunkSize(opts.chunk_size))
            .small_match(opts.small_match)
            .compression_level(opts.compression_level)
            .buffer_size(opts.buffer_size)
            .compare(io::Cursor::new(&mut p))?;
        Ok(p)
    }

    /// Perform qbspatch.
    pub fn qbspatch(&self, s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
        let patcher = Bspatch::new(p)?;
        let mut t = Vec::with_capacity(patcher.hint_target_size() as usize);
        patcher.apply(s, io::Cursor::new(&mut t))?;
        Ok(t)
    }

    /// Perform qbspatch with options.
    pub fn qbspatch_with(&self, s: &[u8], p: &[u8], opts: QbspatchOptions) -> io::Result<Vec<u8>> {
        let patcher = Bspatch::new(p)?;
        let mut t = Vec::with_capacity(patcher.hint_target_size() as usize);
        patcher
            .buffer_size(opts.buffer_size)
            .delta_min(opts.delta_min)
            .apply(s, io::Cursor::new(&mut t))?;
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
        let dir = self.assets_dir.join("random");
        fs::create_dir_all(dir.as_path())?;
        get_random_caches_in(dir, descs)
    }

    /// Run bsdiff to generate patch if cache does not exist then load the cache.
    pub fn load_cached_patch(&self, sample: &Sample) -> io::Result<Vec<u8>> {
        if fs::metadata(sample.patch.as_path()).is_err() {
            let dir = self.assets_dir.join("bin");
            run_command_in(
                dir,
                "bsdiff",
                &[sample.source.as_os_str(), sample.target.as_os_str(), sample.patch.as_os_str()],
            )?;
        }
        fs::read(sample.patch.as_path())
    }
}

/// The benchmarking context.
pub struct Benchmarking {
    assets_dir: path::PathBuf,
}

impl Benchmarking {
    /// Create new benchmarking context.
    pub fn new(assets_dir: path::PathBuf) -> Self {
        Benchmarking { assets_dir }
    }

    /// Perform qbspatch via internal library calls.
    pub fn qbsdiff(&self, s: &[u8], t: &[u8]) -> io::Result<()> {
        Bsdiff::new(s, t)
            .parallel_scheme(ParallelScheme::Auto)
            .compare(io::sink())?;
        Ok(())
    }

    /// Perform qbsdiff with options.
    pub fn qbsdiff_with(&self, s: &[u8], t: &[u8], opts: QbsdiffOptions) -> io::Result<()> {
        Bsdiff::new(s, t)
            .parallel_scheme(ParallelScheme::ChunkSize(opts.chunk_size))
            .small_match(opts.small_match)
            .compression_level(opts.compression_level)
            .buffer_size(opts.buffer_size)
            .compare(io::sink())?;
        Ok(())
    }

    /// Perform qbspatch via internal library calls.
    pub fn qbspatch(&self, s: &[u8], p: &[u8]) -> io::Result<()> {
        Bspatch::new(p)?.apply(s, io::sink())?;
        Ok(())
    }

    /// Perform qbspatch with options.
    pub fn qbspatch_with(&self, s: &[u8], p: &[u8], opts: QbspatchOptions) -> io::Result<()> {
        Bspatch::new(p)?
            .buffer_size(opts.buffer_size)
            .delta_min(opts.delta_min)
            .apply(s, io::sink())?;
        Ok(())
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
        let dir = self.assets_dir.join("random").join("bench");
        fs::create_dir_all(dir.as_path())?;
        get_random_caches_in(dir, descs)
    }

    /// Run bsdiff to generate patch if cache does not exist then load the cache.
    pub fn load_cached_patch(&self, sample: &Sample) -> io::Result<Vec<u8>> {
        if fs::metadata(sample.patch.as_path()).is_err() {
            if self.should_use_bsdiff(sample) {
                let dir = self.assets_dir.join("bin");
                run_command_in(
                    dir,
                    "bsdiff",
                    &[sample.source.as_os_str(), sample.target.as_os_str(), sample.patch.as_os_str()],
                )?;
            } else {
                let mut patch = Vec::new();
                let source = fs::read(sample.source.as_path())?;
                let target = fs::read(sample.target.as_path())?;
                Bsdiff::new(&source[..], &target[..])
                    .compare(io::Cursor::new(&mut patch))?;
                patch.shrink_to_fit();
                fs::write(sample.patch.as_path(), &patch[..])?;
                return Ok(patch);
            }
        }
        fs::read(sample.patch.as_path())
    }

    fn should_use_bsdiff(&self, sample: &Sample) -> bool {
        sample.patch
            .parent()
            .and_then(|p| p.file_name())
            .map(|d| d == "samples")
            .unwrap_or(false)
    }
}

fn run_bsdiff_in<P: AsRef<Path>>(dir: P, s: &[u8], t: &[u8]) -> io::Result<Vec<u8>> {
    let spath = create_temp(s)?;
    let tpath = create_temp(t)?;
    let ppath = create_temp(b"")?;
    run_command_in(
        dir,
        "bsdiff",
        &[spath.as_os_str(), tpath.as_os_str(), ppath.as_os_str()],
    )?;
    fs::read(ppath)
}

fn run_bspatch_in<P: AsRef<Path>>(dir: P, s: &[u8], p: &[u8]) -> io::Result<Vec<u8>> {
    let spath = create_temp(s)?;
    let tpath = create_temp(b"")?;
    let ppath = create_temp(p)?;
    run_command_in(
        dir,
        "bspatch",
        &[spath.as_os_str(), tpath.as_os_str(), ppath.as_os_str()],
    )?;
    fs::read(tpath)
}

fn run_command_in<P, S>(dir: P, cmd: &str, args: &[S]) -> io::Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
{
    let bin = get_binary_in(dir, cmd)?;
    let success = process::Command::new(bin)
        .args(args)
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .spawn()?
        .wait()?
        .success();
    if !success {
        return Err(io::Error::new(io::ErrorKind::Other, "command execution failed"));
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn get_binary_in<P: AsRef<Path>>(dir: P, name: &str) -> io::Result<path::PathBuf> {
    Ok(dir.as_ref().join(format!("{}.exe", name)))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn get_binary_in<P: AsRef<Path>>(dir: P, name: &str) -> io::Result<path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.as_ref().join(name);
    fs::set_permissions(bin.as_path(), fs::Permissions::from_mode(0o755))?;
    Ok(bin)
}

#[cfg(all(unix, target_os = "macos"))]
fn get_binary_in<P: AsRef<Path>>(dir: P, name: &str) -> io::Result<path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.as_ref().join(name).with_extension("macos");
    fs::set_permissions(bin.as_path(), fs::Permissions::from_mode(0o755))?;
    Ok(bin)
}

/// Test sample.
pub struct Sample {
    pub name: String,
    pub source: path::PathBuf,
    pub target: path::PathBuf,
    pub patch: path::PathBuf,
}

impl Sample {
    /// Load source data.
    pub fn load_source(&self) -> io::Result<Vec<u8>> {
        Ok(fs::read(self.source.as_path())?)
    }

    /// Load target data.
    pub fn load_target(&self) -> io::Result<Vec<u8>> {
        Ok(fs::read(self.target.as_path())?)
    }
}

fn get_samples_in<P: AsRef<Path>>(dir: P) -> io::Result<Vec<Sample>> {
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
        let patch;
        if let (Some(d), Some(n)) = (source.parent(), source.file_stem()) {
            let nbuf = n.to_owned();
            name = nbuf.to_string_lossy().into_owned();

            let mut tbuf = nbuf.clone();
            tbuf.push(".t");
            target = path::PathBuf::from(d).join(tbuf.as_os_str());

            let mut pbuf = nbuf.clone();
            pbuf.push(".p");
            patch = path::PathBuf::from(d).join(pbuf.as_os_str());
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "cannot make target or patch path",
            ));
        }

        if let Err(_) = fs::metadata(target.as_path()) {
            continue;
        }

        samples.push(Sample {
            name,
            source,
            target,
            patch,
        });
    }

    Ok(samples)
}

fn get_random_caches_in<P: AsRef<Path>>(dir: P, descs: &[RandomSample]) -> io::Result<Vec<Sample>> {
    fs::create_dir_all(dir.as_ref())?;

    let mut samples = Vec::new();
    for desc in descs.iter() {
        let source = dir.as_ref().join(format!("{}.s", desc.name));
        let sdata;
        if !exists_file(source.as_path()) {
            match desc.source {
                RandomSource::Bytes(bytes) => {
                    sdata = Vec::from(bytes);
                }
                RandomSource::Random(size) => {
                    sdata = random_bytes(size);
                }
            }
            fs::write(source.as_path(), &sdata[..])?;
        } else {
            sdata = fs::read(source.as_path())?;
        }

        for (id, tdesc) in desc.targets.iter().enumerate() {
            let target = dir.as_ref().join(format!("{}.{}.t", desc.name, tdesc.name(id)));
            if !exists_file(target.as_path()) {
                match tdesc {
                    RandomTarget::Bytes(bytes) =>
                        fs::write(target.as_path(), bytes)?,
                    RandomTarget::Distort(rate) =>
                        fs::write(target.as_path(), &distort(&sdata[..], *rate)[..])?,
                }
            }
            let patch = dir.as_ref().join(format!("{}.{}.p", desc.name, tdesc.name(id)));
            samples.push(Sample {
                name: format!("{}/{}", desc.name, tdesc.name(id)),
                source: source.clone(),
                target,
                patch,
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

impl RandomTarget {
    fn name(&self, id: usize) -> String {
        match self {
            RandomTarget::Bytes(bytes) => format!("{}#bin{}", id, bytes.len()),
            RandomTarget::Distort(rate) => format!("{}#sim{}", id, (rate * 100.0) as u32),
        }
    }
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
            source: Random(256 * 1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        RandomSample {
            name: "rand-1m",
            source: Random(1024 * 1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
        RandomSample {
            name: "rand-8m",
            source: Random(8 * 1024 * 1024),
            targets: vec![
                TBytes(b""),
                Distort(0.0),
                Distort(0.5),
                Distort(1.0),
            ],
        },
    ]
}

/// Default random benchmark samples.
pub fn default_random_bench_samples() -> Vec<RandomSample> {
    use RandomSource::Random;
    use RandomTarget::Distort;

    vec![
        RandomSample {
            name: "rand-512k",
            source: Random(512 * 1024),
            targets: vec![
                Distort(0.05),
                Distort(0.50),
                Distort(0.95),
            ],
        },
        RandomSample {
            name: "rand-4m",
            source: Random(4 * 1024 * 1024),
            targets: vec![
                Distort(0.05),
                Distort(0.50),
                Distort(0.95),
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

fn exists_file<P: AsRef<Path>>(name: P) -> bool {
    if let Ok(meta) = fs::metadata(name) {
        meta.is_file()
    } else {
        false
    }
}
