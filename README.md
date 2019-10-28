qbsdiff
=======

[![crates](https://img.shields.io/badge/crates-1.1.0-blue)](https://crates.io/crates/qbsdiff)
[![docs](https://img.shields.io/badge/docs-1.1.0-blue)](https://docs.rs/qbsdiff)
[![dependency status](https://deps.rs/repo/github/hucsmn/qbsdiff/status.svg)](https://deps.rs/repo/github/hucsmn/qbsdiff)


Fast and memory saving bsdiff 4.x compatible delta compressor and patcher.

Add dependency to `Cargo.toml` under your project:
```toml
[dependencies]
qbsdiff = "1.1"
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
