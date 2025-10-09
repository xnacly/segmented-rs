use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use segmented_rs::list::SegmentedList;

pub fn bench_segmented_list(c: &mut Criterion) {
    // helper: bench with a generic closure and element count
    fn bench_push<T: Clone>(c: &mut Criterion, name: &str, template: T, count: usize) {
        c.bench_function(name, |b| {
            b.iter_batched(
                || SegmentedList::new(),
                |mut list| {
                    for _ in 0..count {
                        list.push(black_box(template.clone()));
                    }
                    black_box(list)
                },
                BatchSize::SmallInput,
            )
        });
    }

    bench_push(c, "segmented_list_push_u64", 123u64, 10_000);

    #[derive(Clone)]
    struct MediumElem([u8; 40]);
    bench_push(c, "segmented_list_push_medium", MediumElem([42; 40]), 1_000);

    #[derive(Clone)]
    struct HeavyElem(Box<[u8]>);
    bench_push(
        c,
        "segmented_list_push_heavy_1MiB",
        HeavyElem(vec![161u8; 1 * 1024 * 1024].into_boxed_slice()),
        50, // 100 × 1 MiB = 100 MB total
    );

    bench_push(
        c,
        "segmented_list_push_very_heavy_10MiB",
        HeavyElem(vec![171u8; 10 * 1024 * 1024].into_boxed_slice()),
        10, // 10 × 10 MiB = 100 MB total
    );
}

criterion_group!(benches, bench_segmented_list);
criterion_main!(benches);
