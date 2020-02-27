#![forbid(unsafe_code)]
use super::utils::*;
use bzip2::write::BzEncoder;
use std::io::{Cursor, Result, Write};
use std::ops::Range;
use suffix_array::SuffixArray;
use rayon::prelude::*;

/// Compression level of the bzip2 compressor.
pub use bzip2::Compression;

/// Max length of the source data.
pub use suffix_array::MAX_LENGTH;

/// Default threshold to determine small exact match.
pub const SMALL_MATCH: usize = 12;

/// Default threshold to determine dismatch.
const DISMATCH_COUNT: usize = 8;

/// Default threshold to enable binary search on suffixing similar bytes.
const LONG_SUFFIX: usize = 256;

/// Default buffer size for delta calculation.
pub const BUFFER_SIZE: usize = 4096;

/// Default compression level.
pub const LEVEL: Compression = Compression::Default;

/// Min chunk size of each parallel job, used internally in
/// `ParallelScheme::Auto`.
const MIN_CHUNK: usize = 256 * 1024;

/// Parallel searching scheme of bsdiff.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParallelScheme {
    /// Never search in parallel.
    Never,

    /// Automatically determine parallel scheme.
    Auto,

    /// Each parallel job works on a chunk no large than given size.
    /// 
    /// The chunk size should be greater than 256 KiB, or it would choose a
    /// larger chunk size to avoid bad quality of patch.
    ChunkSize(usize),

    /// Run no more than `N` parallel jobs.
    NumJobs(usize),
}

/// Fast and memory saving bsdiff 4.x compatible delta compressor for
/// execuatbles.
///
/// Source data size should not be greater than MAX_LENGTH (about 4 GiB).
///
/// Compares source with target and generates patch using the best compression
/// level:
/// ```
/// use std::io;
/// use qbsdiff::{Bsdiff, Compression};
///
/// fn bsdiff(source: &[u8], target: &[u8]) -> io::Result<Vec<u8>> {
///     let mut patch = Vec::new();
///     Bsdiff::new(source, target)
///         .compression_level(Compression::Best)
///         .compare(io::Cursor::new(&mut patch))?;
///     Ok(patch)
/// }
/// ```
pub struct Bsdiff<'s, 't> {
    s: &'s [u8],
    t: &'t [u8],
    scheme: ParallelScheme,
    small: usize,
    dismat: usize,
    longsuf: usize,
    bsize: usize,
    level: Compression,
}

impl<'s, 't> Bsdiff<'s, 't> {
    /// Create new configuration for bsdiff delta compression.
    ///
    /// Panics if the length of source data is greater than MAX_LENGTH.
    pub fn new(source: &'s [u8], target: &'t [u8]) -> Self {
        if source.len() > MAX_LENGTH {
            panic!("source data is too large to be indexed");
        }

        Bsdiff {
            s: source,
            t: target,
            scheme: ParallelScheme::Never,
            small: SMALL_MATCH,
            dismat: DISMATCH_COUNT,
            longsuf: LONG_SUFFIX,
            level: Compression::Default,
            bsize: BUFFER_SIZE,
        }
    }

    /// Set the source data.
    pub fn source(mut self, s: &'s [u8]) -> Self {
        self.s = s;
        self
    }

    /// Set the target data.
    pub fn target(mut self, t: &'t [u8]) -> Self {
        self.t = t;
        self
    }

    /// Set parrallel searching scheme (default is `ParallelScheme::Never`).
    /// Chunk size or thread number should not be zero, or it would
    /// automatically choose a proper number instead.
    /// 
    /// Considering that small chunk size of each parallel job may lead to bad
    /// patch quality, the chunk size is forced to be no less than 256 KiB
    /// internally.
    pub fn parallel_scheme(mut self, mut scheme: ParallelScheme) -> Self {
        use ParallelScheme::*;
        if scheme == ChunkSize(0) || scheme == NumJobs(0) {
            scheme = Auto;
        }
        self.scheme = scheme;
        self
    }

    /// Set the threshold to determine small match (default is `SMALL_MATCH`).
    /// If set to zero, no matches would be skipped.
    pub fn small_match(mut self, sm: usize) -> Self {
        self.small = sm;
        self
    }

    /// Set the threshold to determine dismatch (`dis > 0`, default is `DISMATCH_COUNT`).
    #[allow(unused)]
    fn dismatch_count(mut self, mut dis: usize) -> Self {
        if dis < 1 {
            dis = 1;
        }
        self.dismat = dis;
        self
    }

    /// Set the threshold to determine long repating bytes in target data
    /// (`ls` >= 64, default is `LONG_SUFFIX`).
    #[allow(unused)]
    fn long_suffix(mut self, mut ls: usize) -> Self {
        if ls < 64 {
            ls = 64;
        }
        self.longsuf = ls;
        self
    }

    /// Set the compression level of bzip2 (default is `LEVEL`).
    pub fn compression_level(mut self, lv: Compression) -> Self {
        self.level = lv;
        self
    }

    /// Set the buffer size for delta calculation (`bs >= 128`, default is `BUFFER_SIZE`).
    pub fn buffer_size(mut self, mut bs: usize) -> Self {
        if bs < 128 {
            bs = 128;
        }
        self.bsize = bs;
        self
    }

    /// Start searching matches in target and constructing the patch file.
    ///
    /// The size of patch file would be returned if no error occurs.
    pub fn compare<P: Write>(&self, patch: P) -> Result<u64> {
        use ParallelScheme::*;
        let mut chunk = match self.scheme {
            Never =>
                self.t.len(),
            Auto | NumJobs(0) =>
                div_ceil(self.t.len(), Ord::max(num_cpus::get(), 1)),
            ChunkSize(chunk) =>
                chunk,
            NumJobs(n) =>
                div_ceil(self.t.len(), n),
        };
        chunk = Ord::max(chunk, MIN_CHUNK);

        let sa = SuffixArray::new(self.s);
        if chunk >= self.t.len() {
            let diff = SaDiff::new(self.s, self.t, &sa, self.small, self.dismat, self.longsuf);
            pack(self.s, self.t, diff, patch, self.level, self.bsize)
        } else {
            let diff = ParSaDiff::new(self.s, self.t, &sa, chunk, self.small, self.dismat, self.longsuf);
            let ctrls = diff.compute();
            pack(self.s, self.t, ctrls.into_iter(), patch, self.level, self.bsize)
        }
    }
}

/// Calculate `ceil(x/y)`.
#[inline]
fn div_ceil(x: usize, y: usize) -> usize {
    if x % y == 0 {
        x / y
    } else {
        x / y + 1
    }
}

/// Construct bsdiff 4.x patch file from parts.
fn pack<D, P>(s: &[u8], t: &[u8], diff: D, mut p: P, lv: Compression, bsize: usize) -> Result<u64>
where
    D: Iterator<Item = Control>,
    P: Write,
{
    let mut bz_ctrls = Vec::new();
    let mut bz_delta = Vec::new();
    let mut bz_extra = Vec::new();

    {
        let mut ctrls = BzEncoder::new(Cursor::new(&mut bz_ctrls), lv);
        let mut delta = BzEncoder::new(Cursor::new(&mut bz_delta), lv);
        let mut extra = BzEncoder::new(Cursor::new(&mut bz_extra), lv);

        let mut spos = 0;
        let mut tpos = 0;
        let mut cbuf = [0; 24];

        let mut dat = Vec::with_capacity(bsize);

        for ctl in diff {
            // Write control data.
            encode_int(ctl.add as i64, &mut cbuf[0..8]);
            encode_int(ctl.copy as i64, &mut cbuf[8..16]);
            encode_int(ctl.seek, &mut cbuf[16..24]);
            ctrls.write_all(&cbuf[..])?;

            // Compute and write delta data, using limited buffer `dat`.
            if ctl.add > 0 {
                let mut n = ctl.add;
                while n > 0 {
                    let k = Ord::min(n, bsize as u64) as usize;

                    dat.extend(
                        Iterator::zip(s[spos as usize..].iter(), t[tpos as usize..].iter())
                            .map(|(x, y)| y.wrapping_sub(*x))
                            .take(k),
                    );

                    delta.write_all(&dat[..])?;
                    dat.clear();

                    spos += k as u64;
                    tpos += k as u64;
                    n -= k as u64;
                }
            }

            // Write extra data.
            if ctl.copy > 0 {
                extra.write_all(&t[tpos as usize..(tpos + ctl.copy) as usize])?;
                tpos += ctl.copy;
            }

            spos = spos.wrapping_add(ctl.seek as u64);
        }
        ctrls.flush()?;
        delta.flush()?;
        extra.flush()?;
    }
    bz_ctrls.shrink_to_fit();
    bz_delta.shrink_to_fit();
    bz_extra.shrink_to_fit();

    // Write header (b"BSDIFF40", control size, delta size, target size).
    let mut header = [0; 32];
    let csize = bz_ctrls.len() as u64;
    let dsize = bz_delta.len() as u64;
    let esize = bz_extra.len() as u64;
    let tsize = t.len() as u64;
    header[0..8].copy_from_slice(b"BSDIFF40");
    encode_int(csize as i64, &mut header[8..16]);
    encode_int(dsize as i64, &mut header[16..24]);
    encode_int(tsize as i64, &mut header[24..32]);
    p.write_all(&header[..])?;

    // Write bzipped controls, delta data and extra data.
    p.write_all(&bz_ctrls[..])?;
    p.write_all(&bz_delta[..])?;
    p.write_all(&bz_extra[..])?;
    p.flush()?;

    Ok(32 + csize + dsize + esize)
}

struct ParSaDiff<'s, 't> {
    jobs: Vec<SaDiff<'s, 't>>,
}

impl<'s, 't> ParSaDiff<'s, 't> {
    pub fn new(s: &'s [u8], t: &'t [u8], sa: &'s SuffixArray<'s>, chunk: usize, small: usize, dismat: usize, longsuf: usize) -> Self {
        let jobs = t.chunks(chunk)
                .map(|ti| SaDiff::new(s, ti, sa, small, dismat, longsuf))
                .collect();
        ParSaDiff { jobs }
    }

    pub fn compute(mut self) -> Vec<Control> {
        self.jobs.par_iter_mut()
            .map(|diff| {
                let mut pos = 0u64;
                let mut ctrls = Vec::new();
                for ctl in diff {
                    pos += ctl.add;
                    pos = pos.wrapping_add(ctl.seek as u64);
                    ctrls.push(ctl);
                }
                if pos <= std::i64::MAX as u64 {
                    ctrls.push(Control { add: 0, copy: 0, seek: -(pos as i64) });
                } else {
                    ctrls.push(Control { add: 0, copy: 0, seek: std::i64::MIN });
                    ctrls.push(Control { add: 0, copy: 0, seek: -(pos.wrapping_add(std::i64::MIN as u64) as i64) });
                }
                ctrls
            })
            .flatten()
            .collect()
    }
}

/// The delta compression algorithm based on suffix array (a variant of bsdiff 4.x).
struct SaDiff<'s, 't> {
    s: &'s [u8],
    t: &'t [u8],
    sa: &'s SuffixArray<'s>,

    small: usize,
    dismat: usize,
    longsuf: usize,

    i0: usize,
    j0: usize,
    n0: usize,
    b0: usize,
}

impl<'s, 't> SaDiff<'s, 't> {
    /// Creates new search context.
    pub fn new(s: &'s [u8], t: &'t [u8], sa: &'s SuffixArray<'s>, small: usize, dismat: usize, longsuf: usize) -> Self {
        SaDiff {
            s,
            t,
            sa,
            small,
            dismat,
            longsuf,
            i0: 0,
            j0: 0,
            n0: 0,
            b0: 0,
        }
    }

    #[inline]
    fn previous_state(&self) -> (usize, usize, usize, usize) {
        (self.i0, self.j0, self.n0, self.b0)
    }

    #[inline]
    fn update_state(&mut self, i0: usize, j0: usize, n0: usize, b0: usize) {
        self.i0 = i0;
        self.j0 = j0;
        self.n0 = n0;
        self.b0 = b0;
    }

    /// Searches for the next exact match (i, j, n).
    #[inline]
    fn search_next(&mut self) -> Option<(usize, usize, usize)> {
        // EOF is already scanned.
        if self.j0 == self.t.len() && self.b0 == 0 {
            return None;
        }

        let mut j = self.j0 + self.n0;
        let mut k = j;
        let mut m = 0;
        while j < self.t.len().saturating_sub(self.small) {
            // Finds out a possible exact match.
            let (i, n) = range_to_extent(self.sa.search_lcp(&self.t[j..]));

            // Counts the matched bytes, and determine whether these bytes
            // should be treated as possible similar bytes, or simply as the
            // next exact match.
            while k < j + n {
                let i = self.i0.saturating_add(k - self.j0);
                if i < self.s.len() && self.s[i] == self.t[k] {
                    m += 1;
                }
                k += 1;
            }

            if n == 0 {
                // Match nothing.
                j += 1;
                m = 0;
            } else if m == n || n <= self.small {
                // Skip small matches and non-empty exact matches to speed up
                // searching and improve patch quality.
                j += n;
                m = 0;
            } else if n <= m + self.dismat {
                // Bytes with insufficient dismatches were treated as possible
                // suffixing similar data.
                //
                // The entire match is s[i0-b0..i0+n0+a0] ~= t[j0-b0..j0+n0+a0],
                // where
                //     n0 is the exact match (s[i0..i0+n0] == t[j0..j0+n0]),
                //     b0 is the prefixing similar bytes,
                //     a0 is tyhe sufixing similar bytes.
                //
                // Use binary search to approximately find out a proper skip
                // length for long suffixing similar bytes.
                // Do linear search instead when length is not long enough.
                let next = if n <= self.longsuf {
                    j + 1
                } else {
                    let mut x = 0;
                    let mut y = n;
                    while x < y {
                        let z = x + (y - x) / 2;
                        let (iz, nz) = range_to_extent(self.sa.search_lcp(&self.t[j + z..]));
                        if i + n == iz + nz && j + n == j + z + nz {
                            x = z + 1;
                        } else {
                            y = z;
                        }
                    }
                    j + Ord::max(x, 1)
                };
                let mut i = self.i0.saturating_add(j - self.j0);
                while j < next {
                    if i < self.s.len() && self.s[i] == self.t[j] {
                        m -= 1;
                    }
                    i += 1;
                    j += 1;
                }
            } else {
                // The count of dismatches is sufficient.
                return Some((i, j, n));
            }
        }

        // EOF should be treated as the last exact match.
        Some((self.s.len(), self.t.len(), 0))
    }

    /// Shrinks the gap region between the previous and current exact match by
    /// determining similar bytes. Returns the lengths (a0, b) of similar bytes.
    #[inline]
    fn shrink_gap(&self, i: usize, j: usize) -> (usize, usize) {
        let gap = &self.t[self.j0 + self.n0..j];
        let suffix = &self.s[self.i0 + self.n0..];
        let prefix = &self.s[..i];

        let mut a0 = scan_similar(gap.iter(), suffix.iter());
        let mut b = scan_similar(gap.iter().rev(), prefix.iter().rev());

        // Overlapped.
        if a0 + b > gap.len() {
            let n = a0 + b - gap.len();
            let xs = gap[gap.len() - b..a0].iter();
            let ys = suffix[gap.len() - b..a0].iter();
            let zs = prefix[prefix.len() - b..prefix.len() - b + n].iter();

            let i = scan_divide(xs, ys, zs);
            a0 -= n - i;
            b -= i;
        }

        (a0, b)
    }
}

impl<'s, 't> Iterator for SaDiff<'s, 't> {
    type Item = Control;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, j, n)) = self.search_next() {
            let (i0, j0, n0, b0) = self.previous_state();
            let (a0, b) = self.shrink_gap(i, j);

            // source:
            //     ...(   b0   ,   n0   ,   a0   )...(   b   ,...
            //        ^ spos   ^ i0                ^         ^ i
            //                                     | distance can be negative
            // target:
            //     ...(   b0   ,   n0   ,   a0   ;   copy   )(   b   ,...
            //        ^ tpos   ^ j0              ^ tpos+add          ^ j
            let add = (b0 + n0 + a0) as u64;
            let copy = ((j - b) - (j0 + n0 + a0)) as u64;
            let seek = (i - b).wrapping_sub(i0 + n0 + a0) as isize as i64;

            self.update_state(i, j, n, b);
            Some(Control { add, copy, seek })
        } else {
            None
        }
    }
}

/// Converts Range<usize> to extent (i, n).
#[inline]
fn range_to_extent(range: Range<usize>) -> (usize, usize) {
    let Range { start, end } = range;
    (start, end.saturating_sub(start))
}

/// Scans for the data length of the max simailarity.
#[inline]
fn scan_similar<T: Eq, I: Iterator<Item = T>>(xs: I, ys: I) -> usize {
    let mut i = 0;
    let mut matched = 0;
    let mut max_score = 0;

    for (n, eq) in (1..).zip(xs.zip(ys).map(|(x, y)| x == y)) {
        matched += usize::from(eq);
        let dismatched = n - matched;
        let score = matched.wrapping_sub(dismatched) as isize;
        if score > max_score {
            i = n;
            max_score = score;
        }
    }

    i
}

/// Scans for the dividing point of the overlapping.
#[inline]
fn scan_divide<T: Eq, I: Iterator<Item = T>>(xs: I, ys: I, zs: I) -> usize {
    let mut i = 0;
    let mut y_matched = 0;
    let mut z_matched = 0;
    let mut max_score = 0;

    let eqs = xs.zip(ys).zip(zs).map(|((x, y), z)| (x == y, x == z));
    for (n, (y_eq, z_eq)) in (1..).zip(eqs) {
        y_matched += usize::from(y_eq);
        z_matched += usize::from(z_eq);
        let score = y_matched.wrapping_sub(z_matched) as isize;
        if score > max_score {
            i = n;
            max_score = score;
        }
    }

    i
}
