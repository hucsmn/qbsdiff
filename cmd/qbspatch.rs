#![forbid(unsafe_code)]
use std::fs;
use std::io;
use std::io::prelude::*;
use std::process;

use clap::Parser;
use qbsdiff::Bspatch;

#[derive(Parser, Debug)]
#[clap(
name = "qbspatch",
version = "1.4.0",
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
    let mut source;
    let target: Box<dyn Write>;
    let mut patch;
    if args.source_path == "-" {
        source = Vec::new();
        io::stdin().read_to_end(&mut source)?;
    } else {
        source = fs::read(&args.source_path)?;
    }
    source.shrink_to_fit();
    if args.target_path == "-" {
        target = Box::new(io::stdout());
    } else {
        target = Box::new(fs::File::create(&args.target_path)?);
    }
    patch = fs::read(&args.patch_path)?;
    patch.shrink_to_fit();

    // setup delta patcher
    let mut bspatch = Bspatch::new(&patch[..])?;
    if let Some(buffer_size) = args.buffer_size {
        bspatch = bspatch.buffer_size(buffer_size).delta_min(buffer_size / 4)
    }

    // execute delta patcher
    bspatch.apply(&source[..], target)?;
    Ok(())
}
