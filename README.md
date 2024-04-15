qbsdiff
=======

[![crates](https://img.shields.io/badge/crates-1.4.2-blue)](https://crates.io/crates/qbsdiff)
[![docs](https://img.shields.io/badge/docs-1.4.2-blue)](https://docs.rs/qbsdiff)
[![dependency status](https://deps.rs/repo/github/hucsmn/qbsdiff/status.svg)](https://deps.rs/repo/github/hucsmn/qbsdiff)


Fast and memory saving bsdiff 4.x compatible delta compressor and patcher.

Add dependency to `Cargo.toml`:
```toml
[dependencies]
qbsdiff = "1.4"
```

Build commands
--------------

Build `qbsdiff` and `qbspatch` commands:
```shell
$ cargo build --release --bins --features cmd
$ cd target/release
$ ./qbsdiff --help
$ ./qbspatch --help
```
Install commands to `$CARGO_HOME/bin`:
```shell
$ cargo install qbsdiff --features cmd
```

Examples
--------

Produce the target stream by applying `patch` to `source`:
```rust
use std::io;
use qbsdiff::Bspatch;

fn bspatch(source: &[u8], patch: &[u8]) -> io::Result<Vec<u8>> {
    let patcher = Bspatch::new(patch)?;
    let mut target = Vec::new();
    // To preallocate target:
    //Vec::with_capacity(patcher.hint_target_size() as usize);
    patcher.apply(source, io::Cursor::new(&mut target))?;
    Ok(target)
}
```


Produce the patch data by comparing `source` with `target`:
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
