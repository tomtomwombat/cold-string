use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use bench::*;
use cold_string::ColdString;

const SHORT: &str = "qwerty";
const LONG: &str = "this_is_a_longer_string_that_will_allocate";

const LENGTHS: &[usize] = &[4, 8, 16, 32, 64];

fn bench_construction_inner<T: FromStr>(
    g: &mut BenchmarkGroup<'_, WallTime>,
    name: &'static str,
    min: usize,
    max: usize,
    strings: &[String],
) {
    let label = format!("{}-len={}-{}", name, min, max);
    g.bench_function(&label, |b| {
        b.iter(|| {
            for x in strings.iter() {
                let _ = black_box(T::from_str(black_box(x.as_str())));
            }
        })
    });
}

#[rustfmt::skip]
fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("construction");
    for len in LENGTHS {
        for min in [0, *len] {
            let mut strings = Vec::with_capacity(1000);
            for _ in 0..1000 {
                strings.push(random_string(min, *len));
            }
            bench_construction_inner::<String>(&mut group, "std", min, *len, &strings);
            bench_construction_inner::<smol_str::SmolStr>(&mut group, "smol_str", min, *len, &strings);
            bench_construction_inner::<compact_str::CompactString>(&mut group, "compact_str", min, *len, &strings);
            bench_construction_inner::<smartstring::alias::String>(&mut group, "smartstring", min, *len, &strings);
            bench_construction_inner::<smallstr::SmallString<[u8; 8]>>(&mut group, "smallstr", min, *len, &strings);
            bench_construction_inner::<compact_string::CompactString>(&mut group, "compact_string", min, *len, &strings);
            bench_construction_inner::<cold_string::ColdString>(&mut group, "cold-string", min, *len, &strings);
        }
    }
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

fn bench_as_str_inner<T: FromStr + AsRef<str>>(
    g: &mut BenchmarkGroup<'_, WallTime>,
    name: &'static str,
    min: usize,
    max: usize,
    strings: &[String],
    indices: &[usize], // Pass pre-shuffled indices
) {
    // Pre-convert to the target type
    let strings: Vec<_> = strings
        .iter()
        .map(|s| T::from_str(s).map_err(|_| ()).unwrap())
        .collect();

    let strings = black_box(strings);
    let label = format!("{}-len={}-{}", name, min, max);

    g.bench_function(&label, |b| {
        b.iter(|| {
            let mut sum: u8 = 0;
            // Iterate using the shuffled indices to force cache misses
            for &i in indices.iter() {
                let s = &strings[i];
                let x: &str = black_box(s).as_ref();
                // Accessing the data is crucial to force the dereference
                sum ^= x.as_bytes().first().unwrap_or(&0);
            }
            black_box(sum)
        })
    });
}

#[rustfmt::skip]
fn bench_as_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("as_str");
    let count = 1_000_000;
    
    // Pre-calculate random indices once to keep the comparison fair across crates
    let mut indices: Vec<usize> = (0..count).collect();
    fastrand::shuffle(&mut indices);
    // Limit to a subset if 1M iterations inside b.iter is too slow
    let indices_subset = &indices[..1000]; 

    for len in LENGTHS {
        for min in [0, *len] {
            let mut strings = Vec::with_capacity(count);
            for _ in 0..count {
                strings.push(random_string(min, *len));
            }

            bench_as_str_inner::<String>(&mut group, "std", min, *len, &strings, indices_subset);
            bench_as_str_inner::<smol_str::SmolStr>(&mut group, "smol_str", min, *len, &strings, indices_subset);
            bench_as_str_inner::<compact_str::CompactString>(&mut group, "compact_str", min, *len, &strings, indices_subset);
            bench_as_str_inner::<smartstring::alias::String>(&mut group, "smartstring", min, *len, &strings, indices_subset);
            bench_as_str_inner::<smallstr::SmallString<[u8; 8]>>(&mut group, "smallstr", min, *len, &strings, indices_subset);
            bench_as_str_inner::<compact_string::CompactString>(&mut group, "compact_string", min, *len, &strings, indices_subset);
            bench_as_str_inner::<cold_string::ColdString>(&mut group, "cold-string", min, *len, &strings, indices_subset);
        }
    }
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
