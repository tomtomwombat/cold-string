use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::str::FromStr;

use bench::*;

const COUNT: usize = 1000;
const LENGTHS: &[usize] = &[8, 16, 32, 300];
const RATIOS: &[f64] = &[0.0, 0.05, 0.5, 1.0];

fn build_pairs<T>(min: usize, len: usize, eq_ratio: f64) -> (Vec<T>, Vec<T>)
where
    T: FromStr + Clone,
{
    let left: Vec<T> = (0..COUNT).map(|_| random_string::<T>(min, len)).collect();
    let mut right = Vec::with_capacity(COUNT);
    for s in left.iter() {
        if fastrand::f64() < eq_ratio {
            right.push(s.clone());
        } else {
            right.push(random_string::<T>(min, len));
        }
    }
    (left, right)
}

fn bench_eq<T: FromStr + PartialEq + StringType + Clone>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{}::eq", T::name()));

    for &len in LENGTHS.into_iter().filter(|x| **x <= T::max_len()) {
        for min in [0, len] {
            for &ratio in RATIOS {
                let (left, right) = build_pairs::<T>(min, len, ratio);
                let id = BenchmarkId::new(format!("len={}..={},eq={}", min, len, ratio), "");
                group.bench_with_input(id, &(len, ratio), |b, _| {
                    b.iter(|| {
                        for (l, r) in left.iter().zip(right.iter()) {
                            black_box(l == r);
                        }
                    })
                });
            }
        }
    }

    group.finish();
}

fn run_bench_eq(c: &mut Criterion) {
    bench_eq::<String>(c);
    bench_eq::<cold_string::ColdString>(c);
    bench_eq::<compact_string::CompactString>(c);
    bench_eq::<compact_str::CompactString>(c);
    bench_eq::<smartstring::alias::String>(c);
    bench_eq::<smallstr::SmallString<[u8; 8]>>(c);
    bench_eq::<smol_str::SmolStr>(c);
}

criterion_group!(benches, run_bench_eq);
criterion_main!(benches);
