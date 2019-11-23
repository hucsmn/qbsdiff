use byteorder::{ByteOrder, LE};

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
    if x >> 63 == 0 || x == 1 << 63 {
        x as i64
    } else {
        ((x & ((1 << 63) -1)) as i64).wrapping_neg()
    }
}

/// Encodes integer.
#[inline]
pub fn encode_int(x: i64, b: &mut [u8]) {
    if x < 0 {
        LE::write_u64(b, x.wrapping_neg() as u64 | (1 << 63));
    } else {
        LE::write_u64(b, x as u64);
    }
}
