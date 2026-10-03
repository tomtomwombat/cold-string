use bench::*;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
};
use std::hint::black_box;
use std::str::FromStr;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

fn bench_string_type<T>(group: &mut BenchmarkGroup<WallTime>, type_name: &str)
where
    T: FromStr + Clone + Send + Sync,
{
    let threads = 4;

    // Dial for contention:
    // 1024 = Light (very few atomic collisions)
    // 16 = Medium (frequent collisions)
    // 1 = Heavy (4 cores hammering the exact same atomic counter)
    for pool_size in [1024, 16, 1] {
        group.bench_with_input(
            BenchmarkId::new(type_name, pool_size),
            &pool_size,
            |b, &size| {
                let pool: Vec<T> = (0..size)
                    .map(|i| {
                        T::from_str(&format!("test_string_data_for_index_{:010}", i))
                            .map_err(|_| ())
                            .unwrap()
                    })
                    .collect();
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
            },
        );
    }
}

fn bench_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("Clone Drop Contention");
    bench_string_type::<cold_string::ArcColdString>(&mut group, "cold_string::ArcColdString");
    bench_string_type::<cold_string::ArcColdString32>(&mut group, "cold_string::ArcColdString32");
    bench_string_type::<arcstr::ArcStr>(&mut group, "arcstr::ArcStr");
    bench_string_type::<StdArcStr>(&mut group, "Arc<str>");
    group.finish();
}

criterion_group!(benches, bench_contention);
criterion_main!(benches);
