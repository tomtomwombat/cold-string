use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cold_string::ColdString;

const SHORT: &str = "qwerty";
const LONG: &str = "this_is_a_longer_string_that_will_allocate";

fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("construction");

    group.bench_function("ColdString short", |b| {
        b.iter(|| black_box(ColdString::from(black_box(SHORT))))
    });

    group.bench_function("String short", |b| {
        b.iter(|| black_box(String::from(black_box(SHORT))))
    });

    group.bench_function("ColdString long", |b| {
        b.iter(|| black_box(ColdString::from(black_box(LONG))))
    });

    group.bench_function("String long", |b| {
        b.iter(|| black_box(String::from(black_box(LONG))))
    });

    group.finish();
}

fn bench_len(c: &mut Criterion) {
    let cold = ColdString::from(LONG);
    let string = String::from(LONG);

    let mut group = c.benchmark_group("len");

    group.bench_function("ColdString len", |b| b.iter(|| black_box(cold.len())));

    group.bench_function("String len", |b| b.iter(|| black_box(string.len())));

    group.finish();
}

fn bench_as_str(c: &mut Criterion) {
    let cold = ColdString::from(LONG);
    let string = String::from(LONG);

    let mut group = c.benchmark_group("as_str");

    group.bench_function("ColdString as_str", |b| b.iter(|| black_box(cold.as_str())));

    group.bench_function("String as_str", |b| b.iter(|| black_box(string.as_str())));

    group.finish();
}

fn bench_hash(c: &mut Criterion) {
    let cold_long = ColdString::from(LONG);
    let cold_short = ColdString::from(SHORT);
    let string_long = String::from(LONG);
    let string_short = String::from(SHORT);

    let mut group = c.benchmark_group("hash");

    group.bench_function("ColdString hash short", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            cold_short.hash(&mut hasher);
            black_box(hasher.finish());
        })
    });

    group.bench_function("ColdString hash long", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            cold_long.hash(&mut hasher);
            black_box(hasher.finish());
        })
    });

    group.bench_function("String hash short", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            string_short.hash(&mut hasher);
            black_box(hasher.finish());
        })
    });

    group.bench_function("String hash", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            string_long.hash(&mut hasher);
            black_box(hasher.finish());
        })
    });

    group.finish();
}

fn bench_clone(c: &mut Criterion) {
    let cold = ColdString::from(LONG);
    let string = String::from(LONG);

    let mut group = c.benchmark_group("clone");
    group.bench_function("ColdString clone", |b| b.iter(|| black_box(cold.clone())));
    group.bench_function("String clone", |b| b.iter(|| black_box(string.clone())));

    group.finish();
}

criterion_group!(
    benches,
    bench_construction,
    bench_len,
    bench_as_str,
    bench_hash,
    bench_clone
);
criterion_main!(benches);
