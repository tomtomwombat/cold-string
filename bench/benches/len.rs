use bench::*;
use cold_string::ColdString;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fmt::Debug;
use std::hint::black_box;
use std::str::FromStr;

const LENGTHS: &[usize] = &[4, 8, 16, 255];

macro_rules! bench_len_type {
    ($c:expr, $name:expr, $t:ty) => {{
        let mut group = $c.benchmark_group($name);
        for &len in LENGTHS {
            let s: $t = random_string::<String>(len, len).parse().unwrap();
            let id = BenchmarkId::new(format!("len={}", len), "");
            group.bench_with_input(id, &len, |b, _| b.iter(|| black_box(s.len())));
        }
        group.finish();
    }};
}

fn bench_eq(c: &mut Criterion) {
    bench_len_type!(c, "ColdString_len", ColdString);
    bench_len_type!(c, "CompactString_len", compact_string::CompactString);
    bench_len_type!(c, "String_len", String);
}

criterion_group!(benches, bench_eq);
criterion_main!(benches);
