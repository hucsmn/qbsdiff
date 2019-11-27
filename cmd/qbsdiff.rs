#![forbid(unsafe_code)]
use qbsdiff::Bsdiff;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::str::FromStr;

#[macro_use]
extern crate clap;

fn main() {
    let matches = clap_app!(
        qbsdiff =>
        (version: "1.2.0")
        (about: "fast and memory saving bsdiff 4.x compatible delta compressor")
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

    let bsize_expr = matches.value_of("BSIZE").unwrap_or("4096");
    let small_expr = matches.value_of("SMALL").unwrap_or("12");
    let source_name = matches.value_of("SOURCE").unwrap();
    let target_name = matches.value_of("TARGET").unwrap();
    let patch_name = matches.value_of("PATCH").unwrap();

    match BsdiffApp::new(bsize_expr, small_expr, source_name, target_name, patch_name) {
        Ok(app) => {
            if let Err(e) = app.execute() {
                eprintln!("error: {}", e);
            }
        }
        Err(e) => eprintln!("error: {}", e),
    }
}

struct BsdiffApp {
    source: Vec<u8>,
    target: Vec<u8>,
    patch: Box<dyn Write>,
    bsize: usize,
    small: usize,
}

impl BsdiffApp {
    pub fn new(
        bsize_expr: &str,
        small_expr: &str,
        source_name: &str,
        target_name: &str,
        patch_name: &str,
    ) -> io::Result<Self> {
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
            bsize,
            small,
        })
    }

    pub fn execute(self) -> io::Result<()> {
        Bsdiff::new(&self.source[..], &self.target[..])
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
