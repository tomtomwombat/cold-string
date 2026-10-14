use arcstr::ArcStr;
use bench::{StdArcStr, StringType};
use cold_string::ArcColdString;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup, BenchmarkId,
    Criterion,
};
use std::{hint::black_box, str::FromStr};

const LENGTHS: &[usize] = &[16, 128, 512];

#[derive(Clone, Copy)]
enum Operation {
    New,
    Clone,
    Access,
    Drop,
}

fn bench_type<T>(group: &mut BenchmarkGroup<'_, WallTime>, operation: Operation)
where
    T: FromStr + Clone + StringType + AsRef<str> + 'static,
{
    for &len in LENGTHS {
        let text = "x".repeat(len);
        let value = T::from_str(&text).map_err(|_| ()).unwrap();
        let id = BenchmarkId::new(T::name(), len);
        match operation {
            Operation::New => group.bench_with_input(id, &text, |b, text| {
                b.iter_batched(
                    || (),
                    |_| T::from_str(black_box(text)),
                    BatchSize::SmallInput,
                )
            }),
            Operation::Clone => group.bench_with_input(id, &value, |b, value| {
                b.iter_batched(|| (), |_| T::clone(black_box(value)), BatchSize::SmallInput)
            }),
            Operation::Access => group.bench_with_input(id, &value, |b, value| {
                b.iter(|| {
                    let s = black_box(value).as_ref();
                    black_box(s.as_bytes()[len / 2])
                })
            }),
            Operation::Drop => group.bench_with_input(id, &value, |b, value| {
                b.iter_batched(
                    || T::clone(value),
                    |clone| drop(black_box(clone)),
                    BatchSize::SmallInput,
                )
            }),
        };
    }
}

fn bench_operation(c: &mut Criterion, group_name: &'static str, operation: Operation) {
    let mut group = c.benchmark_group(group_name);
    bench_type::<StdArcStr>(&mut group, operation);
    bench_type::<ArcStr>(&mut group, operation);
    bench_type::<ArcColdString>(&mut group, operation);
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
