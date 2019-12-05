/*! Benchmarking diff/patch via invoking this `qbsdiff` crate. */

use criterion::{criterion_group, criterion_main, Criterion};
use std::path;
use std::time;
use utils::*;

pub fn patch(crit: &mut Criterion) {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let benching = Benchmarking::new(assets);

    let descs = default_random_bench_samples();
    let regular = benching.get_regular_samples().unwrap();
    let pathological = benching.get_pathological_samples().unwrap();
    let random = benching.get_random_samples(&descs[..]).unwrap();

    for sample in regular
        .iter()
        .chain(pathological.iter())
        .chain(random.iter())
    {
        let bench_name = format!("patch {}", sample.name);
        let s = sample.load_source().unwrap();
        let p = benching.load_cached_patch(sample).unwrap();
        crit.bench_function(bench_name.as_str(), |b| {
            b.iter(|| benching.qbspatch(&s[..], &p[..]).unwrap())
        });
    }
}

pub fn diff(crit: &mut Criterion) {
    let assets = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let benching = Benchmarking::new(assets);

    let descs = default_random_bench_samples();
    let regular = benching.get_regular_samples().unwrap();
    let pathological = benching.get_pathological_samples().unwrap();
    let random = benching.get_random_samples(&descs[..]).unwrap();

    for sample in regular
        .iter()
        .chain(pathological.iter())
        .chain(random.iter())
    {
        let bench_name = format!("diff {}", sample.name);
        let s = sample.load_source().unwrap();
        let t = sample.load_target().unwrap();
        crit.bench_function(bench_name.as_str(), |b| {
            b.iter(|| benching.qbsdiff(&s[..], &t[..]).unwrap())
        });
    }
}

criterion_group! {
    name = patch_benches;
    config = Criterion::default()
        .sample_size(50)
        .noise_threshold(0.02)
        .warm_up_time(time::Duration::from_millis(500));
    targets = patch,
}

criterion_group! {
    name = diff_benches;
    config = Criterion::default()
        .sample_size(10)
        .noise_threshold(0.02)
        .warm_up_time(time::Duration::from_millis(500));
    targets = diff,
}

criterion_main!(patch_benches, diff_benches);
