#![forbid(unsafe_code)]

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
    let x = u64::from_le_bytes(b[..8].try_into().unwrap());
    if x >> 63 == 0 || x == 1 << 63 {
        x as i64
    } else {
        ((x & ((1 << 63) - 1)) as i64).wrapping_neg()
    }
}

/// Encodes integer.
#[inline]
pub fn encode_int(x: i64, b: &mut [u8]) {
    let n = if x < 0 {
        x.wrapping_neg() as u64 | (1 << 63)
    } else {
        x as u64
    };

    let buf = n.to_le_bytes();
    b[..8].copy_from_slice(&buf);
}
