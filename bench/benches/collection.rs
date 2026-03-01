use ahash::AHashSet;
use bench::*;
use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use std::hash::Hash;
use std::str::FromStr;

const LENGTHS: &[usize] = &[64];

fn bench_hashset_inner<T: FromStr + Eq + Hash>(
    g: &mut BenchmarkGroup<'_, WallTime>,
    name: &'static str,
    min: usize,
    max: usize,
    string_array: &[String],
    indices: &[usize],
) {
    let string_vec: Vec<_> = string_array
        .iter()
        .map(|s| T::from_str(s).map_err(|_| ()).unwrap())
        .collect();
    let strings: AHashSet<_> = string_array
        .iter()
        .map(|s| T::from_str(s).map_err(|_| ()).unwrap())
        .collect();

    let strings = black_box(strings);
    let label = format!("{}-len={}-{}", name, min, max);

    g.bench_function(&label, |b| {
        b.iter(|| {
            for &i in indices.iter() {
                let s = &string_vec[i];
                let _ = black_box(strings.contains(s));
            }
        })
    });
}

#[rustfmt::skip]
fn bench_hashset(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashset");
    let count = 1_000_000;
    
    let mut indices: Vec<usize> = (0..count).collect();
    fastrand::shuffle(&mut indices);
    let indices_subset = &indices[..1000]; 

    for len in LENGTHS {
        for min in [0, *len] {
            let mut strings = Vec::with_capacity(count);
            for _ in 0..count {
                strings.push(random_string(min, *len));
            }
            bench_hashset_inner::<String>(&mut group, "std", min, *len, &strings, indices_subset);
            bench_hashset_inner::<smol_str::SmolStr>(&mut group, "smol_str", min, *len, &strings, indices_subset);
            bench_hashset_inner::<compact_str::CompactString>(&mut group, "compact_str", min, *len, &strings, indices_subset);
            bench_hashset_inner::<smartstring::alias::String>(&mut group, "smartstring", min, *len, &strings, indices_subset);
            bench_hashset_inner::<smallstr::SmallString<[u8; 8]>>(&mut group, "smallstr", min, *len, &strings, indices_subset);
            bench_hashset_inner::<compact_string::CompactString>(&mut group, "compact_string", min, *len, &strings, indices_subset);
            bench_hashset_inner::<cold_string::ColdString>(&mut group, "cold-string", min, *len, &strings, indices_subset);
        }
    }
    group.finish();
}

criterion_group!(benches, bench_hashset);
criterion_main!(benches);
