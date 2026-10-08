use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{hint::black_box, str::FromStr};

use bench::*;

const COUNT: usize = 1000;
const LENGTHS: &[usize] = &[4, 8, 16, 32, 255];

fn bench_new<T: FromStr + StringType>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{}::new", T::name()));

    for &len in LENGTHS {
        for min in [0, len] {
            let strings: Vec<String> = (0..COUNT).map(|_| random_string(min, len)).collect();
            let id = BenchmarkId::new(format!("len={}..={}", min, len), "");
            group.bench_with_input(id, &(min, len), |b, _| {
                b.iter(|| {
                    for x in strings.iter() {
                        let _ = black_box(T::from_str(black_box(x)));
                    }
                })
            });
        }
    }

    group.finish();
}

fn run_bench_new(c: &mut Criterion) {
    bench_new::<String>(c);
    bench_new::<cold_string::ColdString>(c);
    bench_new::<compact_string::CompactString>(c);
    bench_new::<compact_str::CompactString>(c);
    bench_new::<smartstring::alias::String>(c);
    bench_new::<smallstr::SmallString<[u8; 8]>>(c);
    bench_new::<smol_str::SmolStr>(c);
}

criterion_group!(benches, run_bench_new);
criterion_main!(benches);
