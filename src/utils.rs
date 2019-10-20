use byteorder::{ByteOrder, LE};
use std::ops::Range;

/// Single bsdiff control instruction.
#[derive(Debug)]
pub struct Control {
    pub add: u64,
    pub copy: u64,
    pub seek: i64,
}

/// Decodes integer.
#[inline]
pub fn decode_int(b: &[u8]) -> i64 {
    let x = LE::read_u64(b);
    if x >> 63 == 0 || x == 0x8000000000000000 {
        x as i64
    } else {
        ((x & 0x7fffffffffffffff) as i64).wrapping_neg()
    }
}

/// Encodes integer.
#[inline]
pub fn encode_int(x: i64, b: &mut [u8]) {
    if x < 0 {
        LE::write_u64(b, x.wrapping_neg() as u64 | 0x8000000000000000);
    } else {
        LE::write_u64(b, x as u64);
    }
}

/// Converts Range<usize> to extent (i, n).
#[inline]
pub fn range_to_extent(range: Range<usize>) -> (usize, usize) {
    let Range { start, end } = range;
    (start, end.saturating_sub(start))
}
