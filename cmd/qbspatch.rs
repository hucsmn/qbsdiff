#![forbid(unsafe_code)]
use std::io::prelude::*;
use std::{fs, io, process};

use clap::Parser;
use qbsdiff::Bspatch;

#[derive(Parser, Debug)]
#[clap(
name = "qbspatch",
version = "1.4.3",
about = "fast and memory saving bsdiff 4.x compatible patcher",
long_about = None,
)]
struct BspatchArgs {
    /// source file
    #[clap(value_name = "SOURCE")]
    source_path: String,

    /// target file
    #[clap(value_name = "TARGET")]
    target_path: String,

    /// patch file
    #[clap(value_name = "PATCH")]
    patch_path: String,

    /// buffer size
    #[clap(short = 'b', value_name = "BUFFER")]
    buffer_size: Option<usize>,
}

fn main() {
    let args = BspatchArgs::parse();
    if let Err(e) = execute(args) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

fn execute(args: BspatchArgs) -> io::Result<()> {
    // setup input/output
    if args.source_path == "-" && args.patch_path == "-" {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "source and patch are both from stdin",
        ));
    }
    let source = input_bytes(&args.source_path)?;
    let target = output_writer(&args.target_path)?;
    let patch = input_bytes(&args.patch_path)?;

    // setup delta patcher
    let mut bspatch = Bspatch::new(patch.as_slice())?;
    if let Some(buffer_size) = args.buffer_size {
        bspatch = bspatch.buffer_size(buffer_size);
        bspatch = bspatch.delta_min(buffer_size / 4);
    }

    // execute delta patcher
    bspatch.apply(source.as_slice(), target)?;
    Ok(())
}

fn input_bytes(path: &str) -> io::Result<Vec<u8>> {
    let mut data;
    if path == "-" {
        data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
    } else {
        data = fs::read(path)?;
    }
    Ok(data)
}

fn output_writer(path: &str) -> io::Result<Box<dyn Write>> {
    if path == "-" {
        Ok(Box::new(io::stdout()))
    } else {
        Ok(Box::new(fs::File::create(path)?))
    }
}
