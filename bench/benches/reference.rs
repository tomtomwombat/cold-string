use std::sync::Arc;

use arcstr::ArcStr;
use cold_string::ArcColdString;
use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup,
    BenchmarkId, Criterion,
};
use string_cache::DefaultAtom;

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

impl RefString for DefaultAtom {
    fn new(s: &str) -> Self {
        Self::from(s)
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
    if !matches!(operation, Operation::New) {
        bench_type::<DefaultAtom>(&mut group, "DefaultAtom", operation);
    }
    group.finish();
}

fn unique_text(len: usize, id: u64) -> String {
    let mut text = "x".repeat(len);
    let suffix = format!("{id:016x}");
    text.replace_range(len - suffix.len().., &suffix);
    text
}

fn bench_intern(c: &mut Criterion) {
    let mut unique = c.benchmark_group("reference/intern_unique");
    for &len in LENGTHS {
        let mut id = 0;
        unique.bench_with_input(BenchmarkId::new("DefaultAtom", len), &len, |b, &len| {
            b.iter_batched(
                || {
                    id += 1;
                    unique_text(len, id)
                },
                |text| {
                    let atom = DefaultAtom::from(black_box(text.as_str()));
                    (text, atom)
                },
                BatchSize::SmallInput,
            )
        });
    }
    unique.finish();

    let mut hit = c.benchmark_group("reference/intern_hit");
    for &len in LENGTHS {
        let text = "x".repeat(len);
        let retained = DefaultAtom::from(text.as_str());
        hit.bench_with_input(BenchmarkId::new("DefaultAtom", len), &text, |b, text| {
            b.iter_batched(
                || (),
                |_| DefaultAtom::from(black_box(text.as_str())),
                BatchSize::SmallInput,
            )
        });
        black_box(retained);
    }
    hit.finish();
}

fn reference(c: &mut Criterion) {
    bench_operation(c, "reference/new", Operation::New);
    bench_intern(c);
    bench_operation(c, "reference/clone", Operation::Clone);
    bench_operation(c, "reference/access", Operation::Access);
    bench_operation(c, "reference/drop", Operation::Drop);
}

criterion_group!(benches, reference);
criterion_main!(benches);
