use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use segmented_rs::list::SegmentedList;

fn bench_segmented_list_push(c: &mut Criterion) {
    let count = 100_000;

    c.bench_function("segmented_list_push", |b| {
        b.iter_batched(
            || SegmentedList::new(),
            |mut list| {
                for i in 0..count {
                    list.push(black_box(i));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_segmented_list_traverse(c: &mut Criterion) {
    let count = 100_000;
    c.bench_function("segmented_list_traverse", |b| {
        b.iter_batched(
            || {
                let mut l = SegmentedList::new();
                for i in 0..count {
                    l.push(i);
                }
                l
            },
            |list| {
                let mut sum = 0;
                for idx in 0..list.len() {
                    sum += *list.get(idx).unwrap();
                }
                black_box(sum);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_segmented_list_push,
    bench_segmented_list_traverse
);
criterion_main!(benches);
