#![forbid(unsafe_code)]
#[macro_use]
extern crate clap;

use std::fs;
use std::io;
use std::io::prelude::*;
use std::process;
use std::str::FromStr;

use qbsdiff::{Bsdiff, ParallelScheme};

fn main() {
    let matches = clap_app!(
        qbsdiff =>
        (version: "1.4.0")
        (about: "fast and memory saving bsdiff 4.x compatible delta compressor")
        (@arg NOPAR:
            -P
            "disable parallel searching")
        (@arg CHUNK:
            -c +takes_value
            "parallel chunk size")
        (@arg COMPRESS:
            -z +takes_value
            "bzip2 compression level (1-9)")
        (@arg BSIZE:
            -b +takes_value
            "buffer size")
        (@arg SMALL:
            -s +takes_value
            "skip small matches")
        (@arg SOURCE:
            +required
            "source file")
        (@arg TARGET:
            +required
            "target file")
        (@arg PATCH:
            +required
            "patch file"))
        .get_matches();

    let parallel = !matches.is_present("NOPAR");
    let chunk_expr = matches.value_of("CHUNK").unwrap_or("1048576");
    let compress_expr = matches.value_of("COMPRESS").unwrap_or("5");
    let bsize_expr = matches.value_of("BSIZE").unwrap_or("4096");
    let small_expr = matches.value_of("SMALL").unwrap_or("12");
    let source_name = matches.value_of("SOURCE").unwrap();
    let target_name = matches.value_of("TARGET").unwrap();
    let patch_name = matches.value_of("PATCH").unwrap();

    match BsdiffApp::new(
        parallel,
        chunk_expr,
        compress_expr,
        bsize_expr,
        small_expr,
        source_name,
        target_name,
        patch_name,
    ) {
        Ok(app) => {
            if let Err(e) = app.execute() {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}

struct BsdiffApp {
    source: Vec<u8>,
    target: Vec<u8>,
    patch: Box<dyn Write>,
    scheme: ParallelScheme,
    level: u32,
    bsize: usize,
    small: usize,
}

impl BsdiffApp {
    pub fn new(
        parallel: bool,
        chunk_expr: &str,
        compress_expr: &str,
        bsize_expr: &str,
        small_expr: &str,
        source_name: &str,
        target_name: &str,
        patch_name: &str,
    ) -> io::Result<Self> {
        let scheme = if parallel {
            if let Ok(chunk_size) = parse_usize(chunk_expr) {
                ParallelScheme::ChunkSize(Ord::max(chunk_size, 256 * 1024))
            } else {
                ParallelScheme::Auto
            }
        } else {
            ParallelScheme::Never
        };

        let level = match parse_usize(compress_expr)? {
            n if (0..=9).contains(&n) => n as u32,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "compression level must be in range 0-9",
                ));
            }
        };

        let bsize = parse_usize(bsize_expr)?;
        let small = parse_usize(small_expr)?;

        if source_name == "-" && target_name == "-" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source and target could not be stdin at the same time",
            ));
        }

        let mut source;
        if source_name == "-" {
            source = Vec::new();
            io::stdin().read_to_end(&mut source)?;
        } else {
            source = fs::read(source_name)?;
        }
        source.shrink_to_fit();

        let mut target;
        if target_name == "-" {
            target = Vec::new();
            io::stdin().read_to_end(&mut target)?;
        } else {
            target = fs::read(target_name)?;
        }
        target.shrink_to_fit();

        let patch: Box<dyn Write>;
        if patch_name == "-" {
            patch = Box::new(io::stdout());
        } else {
            patch = Box::new(fs::File::create(patch_name)?);
        }

        Ok(BsdiffApp {
            source,
            target,
            patch,
            scheme,
            level,
            bsize,
            small,
        })
    }

    pub fn execute(self) -> io::Result<()> {
        Bsdiff::new(&self.source[..], &self.target[..])
            .parallel_scheme(self.scheme)
            .compression_level(self.level)
            .buffer_size(self.bsize)
            .small_match(self.small)
            .compare(self.patch)?;
        Ok(())
    }
}

fn parse_usize(expr: &str) -> io::Result<usize> {
    match usize::from_str(expr) {
        Ok(n) => Ok(n),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
    }
}
