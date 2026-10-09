use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::hint::black_box;

use cold_string::ColdString;

const SHORT: &str = "qwerty";
const LONG: &str = "this_is_a_longer_string_that_will_allocate";

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

criterion_group!(benches, bench_hash);
criterion_main!(benches);
