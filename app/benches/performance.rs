use criterion::{black_box, criterion_group, criterion_main, Criterion};
use altre::buffer::GapBuffer;

fn benchmark_gap_buffer_insert(c: &mut Criterion) {
    c.bench_function("gap_buffer_insert", |b| {
        b.iter(|| {
            let mut buffer = GapBuffer::new();
            for i in 0..1000 {
                buffer.insert(black_box(i), black_box('a')).unwrap();
            }
        });
    });
}

fn benchmark_cursor_movement(c: &mut Criterion) {
    let mut buffer = GapBuffer::new();
    let text = "a".repeat(10000);
    buffer.insert_str(0, &text).unwrap();

    c.bench_function("cursor_movement", |b| {
        b.iter(|| {
            // Test cursor movement performance
            for i in 0..100 {
                black_box(i % text.len());
            }
        });
    });
}

criterion_group!(benches, benchmark_gap_buffer_insert, benchmark_cursor_movement);
criterion_main!(benches);