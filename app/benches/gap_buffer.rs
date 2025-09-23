use altre::buffer::GapBuffer;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("gap_buffer_insert");
    group.bench_function("sequential_insert", |b| {
        b.iter_batched(
            || GapBuffer::new(),
            |mut buffer| {
                for _ in 0..1024 {
                    buffer.insert(buffer.len_chars(), 'a').unwrap();
                }
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(benches, bench_insert_sequential);
criterion_main!(benches);
