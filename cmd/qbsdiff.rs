#![forbid(unsafe_code)]
use std::io::prelude::*;
use std::{fs, io, process};

use clap::{ArgAction, Parser};
use qbsdiff::{Bsdiff, ParallelScheme};

#[derive(Parser, Debug)]
#[clap(
name = "qbsdiff",
version = "1.4.3",
about = "fast and memory saving bsdiff 4.x compatible delta compressor",
long_about = None,
)]
struct BsdiffArgs {
    /// source file
    #[clap(value_name = "SOURCE")]
    source_path: String,

    /// target file
    #[clap(value_name = "TARGET")]
    target_path: String,

    /// patch file
    #[clap(value_name = "PATCH")]
    patch_path: String,

    /// disable parallel searching
    #[clap(short = 'P', default_value_t = true, action = ArgAction::SetFalse)]
    parallel: bool,

    /// parallel chunk size
    #[clap(short = 'c', value_name = "CHUNK")]
    chunk_size: Option<usize>,

    /// buffer size
    #[clap(short = 'b', value_name = "BUFFER")]
    buffer_size: Option<usize>,

    /// bzip2 compression level (1-9)
    #[clap(short = 'z', value_name = "LEVEL")]
    compress_level: Option<u32>,

    /// skip small matches
    #[clap(short = 's', value_name = "SMALL")]
    small_match: Option<usize>,
}

fn main() {
    let args = BsdiffArgs::parse();
    if let Err(e) = execute(args) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

fn execute(args: BsdiffArgs) -> io::Result<()> {
    // validate command line arguments
    if !matches!(args.compress_level, Some(0..=9) | None) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "compression level must be in range 0-9",
        ));
    }

    // setup input/output
    if args.source_path == "-" && args.target_path == "-" {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "source and target are both from stdin",
        ));
    }
    let source = input_bytes(&args.source_path)?;
    let target = input_bytes(&args.target_path)?;
    let patch = output_writer(&args.patch_path)?;

    // setup delta compressor
    let mut bsdiff = Bsdiff::new(source.as_slice(), target.as_slice());
    if args.parallel {
        bsdiff = bsdiff.parallel_scheme(ParallelScheme::Auto);
    } else if let Some(mut chunk_size) = args.chunk_size {
        chunk_size = Ord::max(chunk_size, 256 * 1024);
        bsdiff = bsdiff.parallel_scheme(ParallelScheme::ChunkSize(chunk_size));
    } else {
        bsdiff = bsdiff.parallel_scheme(ParallelScheme::Never);
    }
    if let Some(compress_level) = args.compress_level {
        bsdiff = bsdiff.compression_level(compress_level);
    }
    if let Some(buffer_size) = args.buffer_size {
        bsdiff = bsdiff.buffer_size(buffer_size);
    }
    if let Some(small_match) = args.small_match {
        bsdiff = bsdiff.small_match(small_match);
    }

    // execute delta compressor
    bsdiff.compare(patch)?;
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
