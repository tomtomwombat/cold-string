use std::hint::black_box;
use std::sync::Arc;

use arcstr::ArcStr;
use cold_string::ArcColdString;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup, BenchmarkId,
    Criterion,
};

const LENGTHS: &[usize] = &[16, 128, 512];

trait RefString: AsRef<str> + Clone + 'static {
    fn new(s: &str) -> Self;
}

impl RefString for Arc<str> {
    fn new(s: &str) -> Self {
        Self::from(s)
    }
}

impl RefString for ArcStr {
    fn new(s: &str) -> Self {
        Self::from(s)
    }
}

impl RefString for ArcColdString {
    fn new(s: &str) -> Self {
        Self::new(s)
    }
}

#[derive(Clone, Copy)]
enum Operation {
    New,
    Clone,
    Access,
    Drop,
}

fn bench_type<T: RefString>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    operation: Operation,
) {
    for &len in LENGTHS {
        let text = "x".repeat(len);
        let value = T::new(&text);

        match operation {
            Operation::New => {
                group.bench_with_input(BenchmarkId::new(name, len), &text, |b, text| {
                    b.iter_batched(|| (), |_| T::new(black_box(text)), BatchSize::SmallInput)
                })
            }
            Operation::Clone => {
                group.bench_with_input(BenchmarkId::new(name, len), &value, |b, value| {
                    b.iter_batched(|| (), |_| T::clone(black_box(value)), BatchSize::SmallInput)
                })
            }
            Operation::Access => {
                group.bench_with_input(BenchmarkId::new(name, len), &value, |b, value| {
                    b.iter(|| {
                        let s = black_box(value).as_ref();
                        black_box(s.as_bytes()[len / 2])
                    })
                })
            }
            Operation::Drop => {
                group.bench_with_input(BenchmarkId::new(name, len), &value, |b, value| {
                    b.iter_batched(
                        || T::clone(value),
                        |clone| drop(black_box(clone)),
                        BatchSize::SmallInput,
                    )
                })
            }
        };
    }
}

fn bench_operation(c: &mut Criterion, group_name: &'static str, operation: Operation) {
    let mut group = c.benchmark_group(group_name);
    bench_type::<Arc<str>>(&mut group, "Arc<str>", operation);
    bench_type::<ArcStr>(&mut group, "ArcStr", operation);
    bench_type::<ArcColdString>(&mut group, "ArcColdString", operation);
    group.finish();
}

fn reference(c: &mut Criterion) {
    bench_operation(c, "reference/new", Operation::New);
    bench_operation(c, "reference/clone", Operation::Clone);
    bench_operation(c, "reference/access", Operation::Access);
    bench_operation(c, "reference/drop", Operation::Drop);
}

criterion_group!(benches, reference);
criterion_main!(benches);
