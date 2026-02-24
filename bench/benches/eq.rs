use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use cold_string::ColdString;

const COUNT: usize = 1000;
const LENGTHS: &[usize] = &[2, 4, 8, 16, 32];
const RATIOS: &[f64] = &[0.0, 0.5, 1.0];

fn random_ascii_string(max_len: usize, rng: &mut StdRng) -> String {
    let len = rng.gen_range(0..=max_len);
    (0..len)
        .map(|_| (rng.gen_range(b'a'..=b'z')) as char)
        .collect()
}

fn build_pairs<T>(len: usize, eq_ratio: f64) -> (Vec<T>, Vec<T>)
where
    T: From<String>,
{
    let mut rng = StdRng::seed_from_u64(42);

    let mut left_strings = Vec::with_capacity(COUNT);
    let mut right_strings = Vec::with_capacity(COUNT);

    for _ in 0..COUNT {
        left_strings.push(random_ascii_string(len, &mut rng));
    }

    for i in 0..COUNT {
        if rng.r#gen::<f64>() < eq_ratio {
            right_strings.push(left_strings[i].clone());
        } else {
            right_strings.push(random_ascii_string(len, &mut rng));
        }
    }

    let left = left_strings.into_iter().map(T::from).collect();
    let right = right_strings.into_iter().map(T::from).collect();

    (left, right)
}

fn bench_eq_type<T>(c: &mut Criterion, name: &str)
where
    T: From<String> + PartialEq,
{
    let mut group = c.benchmark_group(name);

    for &len in LENGTHS {
        for &ratio in RATIOS {
            let (left, right) = build_pairs::<T>(len, ratio);

            group.bench_with_input(
                BenchmarkId::new(format!("len={}_eq={}", len, ratio), ""),
                &(len, ratio),
                |b, _| {
                    b.iter(|| {
                        for (l, r) in left.iter().zip(right.iter()) {
                            black_box(l == r);
                        }
                    })
                },
            );
        }
    }

    group.finish();
}

fn bench_eq(c: &mut Criterion) {
    bench_eq_type::<ColdString>(c, "ColdString_eq");
    //bench_eq_type::<String>(c, "String_eq");
}

criterion_group!(benches, bench_eq);
criterion_main!(benches);
