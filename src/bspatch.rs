#![forbid(unsafe_code)]
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
    patch: PatchFile<'p>,
    buffer_size: usize,
    delta_min: usize,
}

impl<'p> Bspatch<'p> {
    /// Parse the patch file and create new patcher configuration.
    ///
    /// Return error if failed to parse the patch header.
    pub fn new(patch: &'p [u8]) -> Result<Self> {
        Ok(Bspatch {
            patch: parse(patch)?,
            buffer_size: BUFFER_SIZE,
            delta_min: DELTA_MIN,
        })
    }

    /// Set the main copy buffer size, (`bs > 128`, default is `BUFFER_SIZE`).
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
    /// This might be deprecated in later version.
    pub fn delta_min(mut self, mut dm: usize) -> Self {
        if dm < 128 {
            dm = 128;
        }
        self.delta_min = dm;
        self
    }

    /// Hint the final target file size, as provided in the patch header.
    pub fn hint_target_size(&self) -> u64 {
        self.patch.tsize
    }

    /// Apply patch to the source data and output the stream of target.
    ///
    /// The target data size would be returned if no error occurs.
    pub fn apply<T: Write>(self, source: &[u8], target: T) -> Result<u64> {
        let ctx = Context::new(self.patch, source, target, self.buffer_size, self.delta_min);
        ctx.apply()
    }
}

struct PatchFile<'a> {
    tsize: u64,
    ctrls: BzDecoder<Cursor<&'a [u8]>>,
    delta: BzDecoder<Cursor<&'a [u8]>>,
    extra: BzDecoder<Cursor<&'a [u8]>>,
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

    let ctrls = BzDecoder::new(Cursor::new(bz_ctrls));
    let delta = BzDecoder::new(Cursor::new(bz_delta));
    let extra = BzDecoder::new(Cursor::new(bz_extra));

    Ok(PatchFile {
        tsize,
        ctrls,
        delta,
        extra,
    })
}

/// Bspatch context.
struct Context<'s, 'p, T>
where
    T: Write,
{
    source: Cursor<&'s [u8]>,
    target: T,

    patch: PatchFile<'p>,

    n: usize,
    buf: Vec<u8>,
    dlt: Vec<u8>,
    ctl: [u8; 24],

    total: u64,
}

impl<'s, 'p, T> Context<'s, 'p, T>
where
    T: Write,
{
    /// Create context.
    pub fn new(
        patch: PatchFile<'p>,
        source: &'s [u8],
        target: T,
        bsize: usize,
        dsize: usize,
    ) -> Self {
        Context {
            source: Cursor::new(source),
            target,
            patch,
            n: 0,
            buf: vec![0; bsize],
            dlt: vec![0; dsize],
            ctl: [0; 24],
            total: 0,
        }
    }

    /// Apply the patch file.
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

    /// Read the next control.
    fn next(&mut self) -> Option<Result<Control>> {
        match read_exact_or_eof(&mut self.patch.ctrls, &mut self.ctl[..]) {
            Ok(0) => return None,
            Err(e) => return Some(Err(e)),
            _ => (),
        }

        let add = decode_int(&self.ctl[0..]) as u64;
        let copy = decode_int(&self.ctl[8..]) as u64;
        let seek = decode_int(&self.ctl[16..]);
        Some(Ok(Control { add, copy, seek }))
    }

    /// Add delta to source and write the result to target.
    fn add(&mut self, mut count: u64) -> Result<()> {
        while count > 0 {
            let k = Ord::min(count, (self.buf.len() - self.n) as u64) as usize;

            self.source.read_exact(&mut self.buf[self.n..self.n + k])?;
            self.reserve_delta(k);
            self.patch.delta.read_exact(&mut self.dlt[..k])?;
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

    /// Copy extra data to target.
    fn copy(&mut self, mut count: u64) -> Result<()> {
        while count > 0 {
            let k = Ord::min(count, (self.buf.len() - self.n) as u64) as usize;

            self.patch
                .extra
                .read_exact(&mut self.buf[self.n..self.n + k])?;
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

    /// Move the cursor on source.
    fn seek(&mut self, offset: i64) -> Result<()> {
        self.source.seek(SeekFrom::Current(offset))?;
        Ok(())
    }

    /// Extend the delta cache if not large enough.
    fn reserve_delta(&mut self, size: usize) {
        if size > self.dlt.len() {
            self.dlt.resize(size, 0);
        }
    }
}

// Read exact buf.len() bytes or reads an EOF, return the count of bytes readed.
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
