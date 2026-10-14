use bench::*;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::str::FromStr;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

fn run<T: FromStr + Clone + Send + Sync + StringType>(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("{}::contention", T::name()));
    let threads = 4;

    // Measure 4 threads sharing strings in a fixed size pool.
    // smaller pool = more contention
    for pool_size in [1024, 16, 1] {
        let id = BenchmarkId::new(format!("pool={}", pool_size), "");
        group.bench_with_input(id, &pool_size, |b, &size| {
            let pool: Vec<T> = (0..size).map(|_| random_string(16, 16)).collect();
            let pool = Arc::new(pool);
            let mask = size - 1;
            b.iter_custom(|iters| {
                let barrier = Arc::new(Barrier::new(threads + 1));

                thread::scope(|s| {
                    for t in 0..threads {
                        let pool_ref = pool.clone();
                        let b = barrier.clone();

                        s.spawn(move || {
                            let mut idx = t;
                            b.wait();

                            for _ in 0..iters {
                                idx = (idx + 1) & mask;
                                let cloned = black_box(pool_ref[idx].clone());
                                drop(black_box(cloned));
                            }

                            b.wait();
                        });
                    }

                    barrier.wait();
                    let start = Instant::now();
                    barrier.wait();
                    start.elapsed()
                })
            });
        });
    }

    group.finish();
}

fn bench_contention(c: &mut Criterion) {
    run::<cold_string::ArcColdString>(c);
    run::<cold_string::ArcColdString32>(c);
    run::<arcstr::ArcStr>(c);
    run::<StdArcStr>(c);
}

criterion_group!(benches, bench_contention);
criterion_main!(benches);
