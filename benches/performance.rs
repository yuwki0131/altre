use altre::buffer::{EditOperations, GapBuffer, TextEditor};
use altre::performance::{Operation, OptimizationConfig, PerformanceMonitor, PerformanceOptimizer};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use std::time::Duration;

/// ギャップバッファのパフォーマンステスト
fn bench_gap_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("gap_buffer");
    group.measurement_time(Duration::from_secs(10));

    // 挿入操作
    group.bench_function("insert_char", |b| {
        b.iter_batched(
            || GapBuffer::new(),
            |mut buffer| {
                for i in 0..1000 {
                    let _ = buffer.insert(i, black_box('a'));
                }
            },
            BatchSize::SmallInput,
        )
    });

    // 削除操作
    group.bench_function("delete_char", |b| {
        b.iter_batched(
            || {
                let mut buffer = GapBuffer::new();
                for i in 0..1000 {
                    let _ = buffer.insert(i, 'a');
                }
                buffer
            },
            |mut buffer| {
                for i in (0..1000).rev() {
                    let _ = buffer.delete(i);
                }
            },
            BatchSize::SmallInput,
        )
    });

    // 大量テキスト挿入
    group.bench_function("insert_large_text", |b| {
        let large_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100);
        b.iter_batched(
            || GapBuffer::new(),
            |mut buffer| {
                let _ = buffer.insert_str(0, black_box(&large_text));
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// テキストエディターのパフォーマンステスト
fn bench_text_editor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_editor");
    group.measurement_time(Duration::from_secs(10));

    // 基本編集操作
    group.bench_function("insert_and_navigate", |b| {
        b.iter_batched(
            || TextEditor::new(),
            |mut editor| {
                for i in 0..100 {
                    editor
                        .insert_char(black_box(((i % 26) as u8 + b'a') as char))
                        .unwrap();
                    if i % 10 == 0 {
                        editor.insert_char('\n').unwrap();
                    }
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// パフォーマンス監視システムのベンチマーク
fn bench_performance_monitoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_monitoring");
    group.measurement_time(Duration::from_secs(5));

    // 監視なし
    group.bench_function("no_monitoring", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                std::hint::black_box(42);
            }
        })
    });

    // 監視あり
    group.bench_function("with_monitoring", |b| {
        b.iter_batched(
            || PerformanceMonitor::new(),
            |mut monitor| {
                for _ in 0..1000 {
                    let timer = monitor.start_operation(Operation::CursorMove);
                    std::hint::black_box(42);
                    timer.finish(&mut monitor);
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// 最適化システムのベンチマーク
fn bench_optimization_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimization");
    group.measurement_time(Duration::from_secs(5));

    // 短い行の戦略決定
    group.bench_function("short_line_strategy", |b| {
        b.iter_batched(
            || PerformanceOptimizer::new(OptimizationConfig::new()),
            |mut optimizer| {
                for i in 0..1000 {
                    optimizer.determine_long_line_strategy(black_box(100), i);
                }
            },
            BatchSize::SmallInput,
        )
    });

    // 長い行の戦略決定
    group.bench_function("long_line_strategy", |b| {
        b.iter_batched(
            || PerformanceOptimizer::new(OptimizationConfig::new()),
            |mut optimizer| {
                for i in 0..100 {
                    optimizer.determine_long_line_strategy(black_box(5000), i);
                }
            },
            BatchSize::SmallInput,
        )
    });

    // ギャップバッファサイズ最適化
    group.bench_function("gap_buffer_optimization", |b| {
        b.iter_batched(
            || PerformanceOptimizer::new(OptimizationConfig::new()),
            |optimizer| {
                for i in 0..1000 {
                    optimizer.optimize_gap_buffer_size(black_box(i * 100), black_box(i));
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    performance_benches,
    bench_gap_buffer_operations,
    bench_text_editor_operations,
    bench_performance_monitoring,
    bench_optimization_system
);
criterion_main!(performance_benches);
