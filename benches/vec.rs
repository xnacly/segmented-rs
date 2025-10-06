use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

fn bench_vec_push(c: &mut Criterion) {
    let count = 100_000;
    c.bench_function("vec_push", |b| {
        b.iter_batched(
            || Vec::new(),
            |mut v| {
                for i in 0..count {
                    v.push(black_box(i));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_vec_traverse(c: &mut Criterion) {
    let count = 100_000;
    c.bench_function("vec_traverse", |b| {
        b.iter_batched(
            || {
                let mut v = Vec::new();
                for i in 0..count {
                    v.push(i);
                }
                v
            },
            |v| {
                let mut sum = 0;
                for x in &v {
                    sum += black_box(*x);
                }
                black_box(sum);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_vec_push, bench_vec_traverse);
criterion_main!(benches);
