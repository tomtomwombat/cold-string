use ahash::AHashSet;
use bench::*;
use criterion::BenchmarkId;
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeSet;
use std::hash::Hash;
use std::hint::black_box;
use std::str::FromStr;

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

fn bench_hash_set<T>(c: &mut Criterion)
where
    T: FromStr + Clone + StringType + PartialEq + Eq + Hash,
{
    let mut group = c.benchmark_group(format!("{}::hash_set", T::name()));

    for &len in LENGTHS.into_iter().filter(|x| **x <= T::max_len()) {
        for min in [0, len] {
            for &ratio in RATIOS {
                let (left, right) = build_pairs::<T>(min, len, ratio);
                let set: AHashSet<T> = left.into_iter().collect();
                let id = BenchmarkId::new(format!("len={}..={},eq={}", min, len, ratio), "");
                group.bench_with_input(id, &(min, len), |b, _| {
                    b.iter(|| {
                        for s in right.iter() {
                            black_box(set.contains(s));
                        }
                    })
                });
            }
        }
    }
    group.finish();
}

fn bench_btree_set<T>(c: &mut Criterion)
where
    T: FromStr + Clone + StringType + PartialEq + Eq + Hash + Ord,
{
    let mut group = c.benchmark_group(format!("{}::btree_set", T::name()));

    for &len in LENGTHS.into_iter().filter(|x| **x <= T::max_len()) {
        for min in [0, len] {
            for &ratio in RATIOS {
                let (left, right) = build_pairs::<T>(min, len, ratio);
                let set: BTreeSet<T> = left.into_iter().collect();
                let id = BenchmarkId::new(format!("len={}..={},eq={}", min, len, ratio), "");
                group.bench_with_input(id, &(min, len), |b, _| {
                    b.iter(|| {
                        for s in right.iter() {
                            black_box(set.contains(s));
                        }
                    })
                });
            }
        }
    }
    group.finish();
}

fn run_bench(c: &mut Criterion) {
    bench_hash_set::<String>(c);
    bench_hash_set::<cold_string::ColdString>(c);
    bench_hash_set::<compact_str::CompactString>(c);
    bench_hash_set::<compact_string::CompactString>(c);
    bench_hash_set::<compact_str::CompactString>(c);
    bench_hash_set::<smartstring::alias::String>(c);
    bench_hash_set::<smallstr::SmallString<[u8; 8]>>(c);
    bench_hash_set::<smol_str::SmolStr>(c);

    bench_btree_set::<String>(c);
    bench_btree_set::<cold_string::ColdString>(c);
    bench_btree_set::<compact_str::CompactString>(c);
    bench_btree_set::<compact_string::CompactString>(c);
    bench_btree_set::<compact_str::CompactString>(c);
    bench_btree_set::<smartstring::alias::String>(c);
    bench_btree_set::<smallstr::SmallString<[u8; 8]>>(c);
    bench_btree_set::<smol_str::SmolStr>(c);
}

criterion_group!(benches, run_bench);
criterion_main!(benches);
