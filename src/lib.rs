/*!
Fast and memory saving delta compressor compatible with bsdiff 4.x.
*/

pub mod bsdiff;
pub mod bspatch;

pub use bsdiff::{Bsdiff, Compression};
pub use bspatch::Bspatch;

/// Single bsdiff control instruction.
#[derive(Debug)]
struct Control {
    pub add: u64,
    pub copy: u64,
    pub seek: i64,
}
