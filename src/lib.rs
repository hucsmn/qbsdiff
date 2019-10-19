/*!
Fast and memory saving bsdiff 4.x compatible delta compressor and patcher.

Add dependency to `Cargo.toml` under your project:
```toml
[dependencies]
qbsdiff = "0.1"
```

Build commands
--------------

The commands `qbsdiff` and `qbspatch` could be compiled with:
```shell
$ cargo build --release --bins --features cmd
$ target/release/qbsdiff -h
$ target/release/qbspatch -h
```

Examples
--------

Apply patch to source and produce the target data:
```rust
use std::io;
use qbsdiff::Bspatch;

fn bspatch(source: &[u8], patch: &[u8]) -> io::Result<Vec<u8>> {
    let patcher = Bspatch::new(patch)?;
    let mut target = Vec::new(); // More complicated: Vec::with_capacity(patcher.hint_target_size() as usize);
    patcher.apply(source, io::Cursor::new(&mut target))?;
    Ok(target)
}
```


Compare source with target then generate patch:
```rust
use std::io;
use qbsdiff::Bsdiff;

fn bsdiff(source: &[u8], target: &[u8]) -> io::Result<Vec<u8>> {
    let mut patch = Vec::new();
    Bsdiff::new(source, target)
        .compare(io::Cursor::new(&mut patch))?;
    Ok(patch)
}
```

Note that `qbsdiff` would not generate exactly the same patch file as `bsdiff`.
Only the patch file format is promised to be compatible.
*/

pub mod bsdiff;
pub mod bspatch;

use byteorder::{ByteOrder, LE};

pub use bsdiff::{Bsdiff, Compression};
pub use bspatch::Bspatch;

/// Single bsdiff control instruction.
#[derive(Debug)]
struct Control {
    pub add: u64,
    pub copy: u64,
    pub seek: i64,
}

/// Decodes integer.
#[inline]
fn decode_int(b: &[u8]) -> i64 {
    let x = LE::read_u64(b);
    if x >> 63 == 0 || x == 0x8000000000000000 {
        x as i64
    } else {
        ((x & 0x7fffffffffffffff) as i64).wrapping_neg()
    }
}

/// Encodes integer.
#[inline]
fn encode_int(x: i64, b: &mut [u8]) {
    if x < 0 {
        LE::write_u64(b, x.wrapping_neg() as u64 | 0x8000000000000000);
    } else {
        LE::write_u64(b, x as u64);
    }
}
