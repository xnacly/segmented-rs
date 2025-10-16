use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

pub fn bench_vec(c: &mut Criterion) {
    // helper: bench with a generic closure and element count
    fn bench_push<T: Clone>(c: &mut Criterion, name: &str, template: T, count: usize) {
        c.bench_function(name, |b| {
            b.iter_batched(
                || Vec::new(),
                |mut vec| {
                    for _ in 0..count {
                        vec.push(black_box(template.clone()));
                    }
                    black_box(vec)
                },
                BatchSize::SmallInput,
            )
        });
    }

    bench_push(c, "vec_push_u64", 123u64, 10_000);

    #[derive(Clone)]
    struct MediumElem([u8; 40]);
    bench_push(c, "vec_push_medium", MediumElem([42; 40]), 1_000);

    #[derive(Clone)]
    struct HeavyElem(Box<[u8]>);
    bench_push(
        c,
        "vec_push_heavy_1MiB",
        HeavyElem(vec![161u8; 1 * 1024 * 1024].into_boxed_slice()),
        10,
    );

    bench_push(
        c,
        "vec_push_heavy_10MiB",
        HeavyElem(vec![161u8; 10 * 1024 * 1024].into_boxed_slice()),
        1,
    );

    bench_push(
        c,
        "vec_push_heavy_50MiB",
        HeavyElem(vec![161u8; 50 * 1024 * 1024].into_boxed_slice()),
        1,
    );
}

criterion_group!(benches, bench_vec);
criterion_main!(benches);
