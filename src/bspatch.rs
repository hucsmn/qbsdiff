use super::utils::*;
use bzip2::read::BzDecoder;
use std::io::{Cursor, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

/// Default buffer size.
pub const BUFFER_SIZE: usize = 16384;

/// Default initial size of the delta calculation buffer.
pub const DELTA_MIN: usize = 1024;

/// Fast and memory saving patcher comaptible with bspatch.
///
/// Apply patch to source using a 4k buffer:
/// ```
/// use std::io;
/// use qbsdiff::Bspatch;
///
/// fn bspatch(source: &[u8], patch: &[u8]) -> io::Result<Vec<u8>> {
///     let mut target = Vec::new();
///     Bspatch::new(patch)?
///         .buffer_size(4096)
///         .apply(source, io::Cursor::new(&mut target))?;
///     Ok(target)
/// }
/// ```
///
/// Preallocate target vector before applying patch:
/// ```
/// use std::io;
/// use qbsdiff::Bspatch;
///
/// fn bspatch(source: &[u8], patch: &[u8]) -> io::Result<Vec<u8>> {
///     let patcher = Bspatch::new(patch)?;
///     let mut target = Vec::with_capacity(patcher.hint_target_size() as usize);
///     patcher.apply(source, io::Cursor::new(&mut target))?;
///     Ok(target)
/// }
/// ```
pub struct Bspatch<'p> {
    ctrls: BzDecoder<Cursor<&'p [u8]>>,
    delta: BzDecoder<Cursor<&'p [u8]>>,
    extra: BzDecoder<Cursor<&'p [u8]>>,
    tsize: u64,
    buffer_size: usize,
    delta_min: usize,
}

impl<'p> Bspatch<'p> {
    /// Parses given bsdiff 4.x patch.
    ///
    /// Returns error if parsing failed.
    pub fn new(patch: &'p [u8]) -> Result<Self> {
        let PatchFile {
            tsize,
            bz_ctrls,
            bz_delta,
            bz_extra,
        } = parse(patch)?;
        let ctrls = BzDecoder::new(Cursor::new(bz_ctrls));
        let delta = BzDecoder::new(Cursor::new(bz_delta));
        let extra = BzDecoder::new(Cursor::new(bz_extra));
        Ok(Bspatch {
            ctrls,
            delta,
            extra,
            tsize,
            buffer_size: BUFFER_SIZE,
            delta_min: DELTA_MIN,
        })
    }

    /// Sets the main copy buffer size, (`bs > 128`, default is `BUFFER_SIZE`).
    pub fn buffer_size(mut self, mut bs: usize) -> Self {
        if bs < 128 {
            bs = 128;
        }
        self.buffer_size = bs;
        self
    }

    /// Sets the initial delta cache size, (`dm > 128`, default is `DELTA_MIN`).
    ///
    /// The delta cache is dynamic and can grow up when needed (but keeps not
    /// greater than the size of main copy buffer).
    ///
    /// This may be deprecated in later versions.
    pub fn delta_min(mut self, mut dm: usize) -> Self {
        if dm < 128 {
            dm = 128;
        }
        self.delta_min = dm;
        self
    }

    /// Hints the final target data size, as provided in the patch header.
    pub fn hint_target_size(&self) -> u64 {
        self.tsize
    }

    /// Applies the patch to source data and outputs the target stream.
    ///
    /// Returns count of bytes writed to target if no error encountered.
    pub fn apply<T: Write>(self, source: &[u8], target: T) -> Result<u64> {
        let ctx = Context::new(
            self.ctrls,
            self.delta,
            self.extra,
            source,
            target,
            self.buffer_size,
            self.delta_min,
        );
        ctx.apply()
    }
}

struct PatchFile<'a> {
    tsize: u64,
    bz_ctrls: &'a [u8],
    bz_delta: &'a [u8],
    bz_extra: &'a [u8],
}

/// Parse the bsdiff 4.x patch file.
fn parse(patch: &[u8]) -> Result<PatchFile> {
    if patch.len() < 32 || &patch[..8] != b"BSDIFF40" {
        return Err(Error::new(ErrorKind::InvalidData, "not a valid patch"));
    }

    let csize = decode_int(&patch[8..16]) as u64;
    let dsize = decode_int(&patch[16..24]) as u64;
    let tsize = decode_int(&patch[24..32]) as u64;
    if 32 + csize + dsize > patch.len() as u64 {
        return Err(Error::new(ErrorKind::InvalidData, "patch corrupted"));
    }

    let (_, remain) = patch.split_at(32);
    let (bz_ctrls, remain) = remain.split_at(csize as usize);
    let (bz_delta, bz_extra) = remain.split_at(dsize as usize);

    Ok(PatchFile {
        tsize,
        bz_ctrls,
        bz_delta,
        bz_extra,
    })
}

/// Bspatch context.
struct Context<'s, T, C, D, E>
where
    T: Write,
    C: Read,
    D: Read,
    E: Read,
{
    source: Cursor<&'s [u8]>,
    target: T,

    ctrls: C,
    delta: D,
    extra: E,

    n: usize,
    buf: Vec<u8>,
    dlt: Vec<u8>,
    ctl: [u8; 24],

    total: u64,
}

impl<'s, T, C, D, E> Context<'s, T, C, D, E>
where
    T: Write,
    C: Read,
    D: Read,
    E: Read,
{
    /// Create context.
    pub fn new(
        ctrls: C,
        delta: D,
        extra: E,
        source: &'s [u8],
        target: T,
        bsize: usize,
        dsize: usize,
    ) -> Self {
        let mut buf = Vec::with_capacity(bsize);
        let mut dlt = Vec::with_capacity(dsize);
        unsafe {
            buf.set_len(bsize);
            dlt.set_len(dsize);
        }

        Context {
            source: Cursor::new(source),
            target,
            ctrls,
            delta,
            extra,
            n: 0,
            buf,
            dlt,
            ctl: [0; 24],
            total: 0,
        }
    }

    /// Apply the bsdiff 4.x patch file.
    pub fn apply(mut self) -> Result<u64> {
        while let Some(result) = self.next() {
            match result {
                Ok(Control { add, copy, seek }) => {
                    self.add(add)?;
                    self.copy(copy)?;
                    self.seek(seek)?;
                }
                Err(e) => return Err(e),
            }
        }
        if self.n > 0 {
            self.target.write_all(&self.buf[..self.n])?;
        }
        self.target.flush()?;
        Ok(self.total)
    }

    /// Reads the next control.
    fn next(&mut self) -> Option<Result<Control>> {
        match read_exact_or_eof(&mut self.ctrls, &mut self.ctl[..]) {
            Ok(0) => return None,
            Err(e) => return Some(Err(e)),
            _ => (),
        }

        let add = decode_int(&self.ctl[0..]) as u64;
        let copy = decode_int(&self.ctl[8..]) as u64;
        let seek = decode_int(&self.ctl[16..]);
        Some(Ok(Control { add, copy, seek }))
    }

    /// Adds delta to source and writes into the target.
    fn add(&mut self, mut count: u64) -> Result<()> {
        while count > 0 {
            let k = Ord::min(count, (self.buf.len() - self.n) as u64) as usize;

            self.source.read_exact(&mut self.buf[self.n..self.n + k])?;
            self.reserve_delta(k);
            self.delta.read_exact(&mut self.dlt[..k])?;
            for i in 0..k {
                let j = self.n + i;
                self.buf[j] = self.buf[j].wrapping_add(self.dlt[i]);
            }
            self.n += k;
            if self.n >= self.buf.len() {
                self.target.write_all(self.buf.as_ref())?;
                self.n = 0;
            }
            self.total += k as u64;
            count -= k as u64;
        }
        Ok(())
    }

    /// Copies extra data to the target.
    fn copy(&mut self, mut count: u64) -> Result<()> {
        while count > 0 {
            let k = Ord::min(count, (self.buf.len() - self.n) as u64) as usize;

            self.extra.read_exact(&mut self.buf[self.n..self.n + k])?;
            self.n += k;
            if self.n >= self.buf.len() {
                self.target.write_all(self.buf.as_ref())?;
                self.n = 0;
            }
            self.total += k as u64;
            count -= k as u64;
        }
        Ok(())
    }

    /// Sets source cursor.
    fn seek(&mut self, offset: i64) -> Result<()> {
        self.source.seek(SeekFrom::Current(offset))?;
        Ok(())
    }

    /// Extend the delta cache if not large enough.
    fn reserve_delta(&mut self, size: usize) {
        if size > self.dlt.len() {
            let n = size - self.dlt.len();
            self.dlt.reserve(n);
            unsafe {
                self.dlt.set_len(size);
            }
        }
    }
}

// Reads exact buf.len() bytes or reads an EOF, returns size of data readed.
#[inline]
fn read_exact_or_eof<R>(r: &mut R, buf: &mut [u8]) -> Result<usize>
where
    R: Read,
{
    let mut cnt = 0;
    while cnt < buf.len() {
        match r.read(&mut buf[cnt..]) {
            Ok(0) => break,
            Ok(n) => cnt += n,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if cnt != 0 && cnt != buf.len() {
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    } else {
        Ok(cnt)
    }
}
