use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{hint::black_box, str::FromStr};

use bench::*;

const COUNT: usize = 1000;
const LENGTHS: &[usize] = &[4, 8, 16, 32, 300];

fn run<T: FromStr + StringType + Clone>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{}::clone", T::name()));

    for &len in LENGTHS.into_iter().filter(|x| **x <= T::max_len()) {
        for min in [0, len] {
            let strings: Vec<T> = (0..COUNT).map(|_| random_string(min, len)).collect();
            let id = BenchmarkId::new(format!("len={}..={}", min, len), "");
            group.bench_with_input(id, &(min, len), |b, _| {
                b.iter(|| {
                    for s in strings.iter() {
                        black_box(black_box(s).clone());
                    }
                })
            });
        }
    }

    group.finish();
}

fn run_bench(c: &mut Criterion) {
    run::<String>(c);
    run::<cold_string::ColdString>(c);
    run::<compact_str::CompactString>(c);
    run::<compact_string::CompactString>(c);
    run::<smartstring::alias::String>(c);
    run::<smallstr::SmallString<[u8; 8]>>(c);
    run::<smol_str::SmolStr>(c);
}

criterion_group!(benches, run_bench);
criterion_main!(benches);
