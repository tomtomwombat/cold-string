use bench::*;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

const LENGTHS: &[usize] = &[4, 8, 16, 255];

macro_rules! bench_len_type {
    ($c:expr, $t:ty) => {{
        let mut group = $c.benchmark_group(format!("{}::len", <$t>::name()));
        for &len in LENGTHS {
            let s: $t = random_string::<String>(len, len).parse().unwrap();
            let id = BenchmarkId::new(format!("len={}", len), "");
            group.bench_with_input(id, &len, |b, _| b.iter(|| black_box(s.len())));
        }
        group.finish();
    }};
}

fn bench_len(c: &mut Criterion) {
    bench_len_type!(c, String);
    bench_len_type!(c, cold_string::ColdString);
    bench_len_type!(c, compact_string::CompactString);
    bench_len_type!(c, compact_str::CompactString);
    bench_len_type!(c, smartstring::alias::String);
    bench_len_type!(c, smallstr::SmallString<[u8; 8]>);
    bench_len_type!(c, smol_str::SmolStr);
}

criterion_group!(benches, bench_len);
criterion_main!(benches);
