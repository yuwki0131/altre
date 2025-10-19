use altre::buffer::{NavigationAction, NavigationSystem};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use std::time::Duration;

/// 基本的なカーソル移動のベンチマーク
fn bench_cursor_movement(c: &mut Criterion) {
    let mut group = c.benchmark_group("cursor_movement");
    group.measurement_time(Duration::from_secs(10));

    // 短いテキスト
    let short_text = "Hello, World!\nThis is a test.";
    group.bench_function("short_text_forward", |b| {
        b.iter_batched(
            || NavigationSystem::new(),
            |mut nav| {
                for _ in 0..100 {
                    let _ = nav.navigate(black_box(short_text), NavigationAction::MoveCharForward);
                }
            },
            BatchSize::SmallInput,
        )
    });

    // 中程度のテキスト
    let medium_text = "a".repeat(1000) + "\n" + &"b".repeat(1000);
    group.bench_function("medium_text_forward", |b| {
        b.iter_batched(
            || NavigationSystem::new(),
            |mut nav| {
                for _ in 0..100 {
                    let _ =
                        nav.navigate(black_box(&medium_text), NavigationAction::MoveCharForward);
                }
            },
            BatchSize::SmallInput,
        )
    });

    // 長いテキスト（1行が非常に長い）
    let long_line_text = "x".repeat(10000);
    group.bench_function("long_line_forward", |b| {
        b.iter_batched(
            || NavigationSystem::new(),
            |mut nav| {
                for _ in 0..10 {
                    let _ = nav.navigate(
                        black_box(&long_line_text),
                        NavigationAction::MoveCharForward,
                    );
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// 高性能ナビゲーションシステムのベンチマーク
fn bench_high_performance_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("high_performance_navigation");
    group.measurement_time(Duration::from_secs(10));

    let long_line_text = "x".repeat(10000);

    group.bench_function("standard_navigation", |b| {
        b.iter_batched(
            || NavigationSystem::new(),
            |mut nav| {
                for _ in 0..10 {
                    let _ = nav.navigate(
                        black_box(&long_line_text),
                        NavigationAction::MoveCharForward,
                    );
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("high_performance_navigation", |b| {
        b.iter_batched(
            || NavigationSystem::with_high_performance(),
            |mut nav| {
                for _ in 0..10 {
                    let _ = nav.navigate(
                        black_box(&long_line_text),
                        NavigationAction::MoveCharForward,
                    );
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// 行単位ナビゲーションのベンチマーク
fn bench_line_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_navigation");
    group.measurement_time(Duration::from_secs(5));

    // 多数の行を持つテキスト
    let multi_line_text: String = (0..1000).map(|i| format!("Line {}\n", i)).collect();

    group.bench_function("line_up_down", |b| {
        b.iter_batched(
            || {
                let mut nav = NavigationSystem::with_performance_monitoring();
                // 中央の行から開始
                for _ in 0..500 {
                    let _ = nav.navigate(&multi_line_text, NavigationAction::MoveLineDown);
                }
                nav
            },
            |mut nav| {
                // 上下移動を繰り返す
                for _ in 0..50 {
                    let _ = nav.navigate(black_box(&multi_line_text), NavigationAction::MoveLineUp);
                    let _ =
                        nav.navigate(black_box(&multi_line_text), NavigationAction::MoveLineDown);
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("line_start_end", |b| {
        b.iter_batched(
            || NavigationSystem::with_performance_monitoring(),
            |mut nav| {
                for _ in 0..100 {
                    let _ =
                        nav.navigate(black_box(&multi_line_text), NavigationAction::MoveLineStart);
                    let _ =
                        nav.navigate(black_box(&multi_line_text), NavigationAction::MoveLineEnd);
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// バッファ全体移動のベンチマーク
fn bench_buffer_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_navigation");
    group.measurement_time(Duration::from_secs(5));

    // 大きなファイルをシミュレート
    let large_file: String = (0..5000)
        .map(|i| format!("Line {} with some content here\n", i))
        .collect();

    group.bench_function("buffer_start_end", |b| {
        b.iter_batched(
            || NavigationSystem::with_performance_monitoring(),
            |mut nav| {
                for _ in 0..50 {
                    let _ = nav.navigate(black_box(&large_file), NavigationAction::MoveBufferStart);
                    let _ = nav.navigate(black_box(&large_file), NavigationAction::MoveBufferEnd);
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Tab幅計算のベンチマーク
fn bench_tab_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab_navigation");
    group.measurement_time(Duration::from_secs(5));

    // タブを含むテキスト
    let tab_text = "function test() {\n\treturn 'hello';\n}\n".repeat(1000);

    group.bench_function("tab_width_4", |b| {
        b.iter_batched(
            || NavigationSystem::with_performance_monitoring(),
            |mut nav| {
                for _ in 0..100 {
                    let _ = nav.navigate_with_tab_width(
                        black_box(&tab_text),
                        NavigationAction::MoveCharForward,
                        4,
                    );
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("tab_width_8", |b| {
        b.iter_batched(
            || NavigationSystem::with_performance_monitoring(),
            |mut nav| {
                for _ in 0..100 {
                    let _ = nav.navigate_with_tab_width(
                        black_box(&tab_text),
                        NavigationAction::MoveCharForward,
                        8,
                    );
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// パフォーマンス目標の検証
fn bench_performance_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_targets");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(1000);

    // カーソル移動が1ms未満であることを検証
    let short_text = "Hello, World!";
    group.bench_function("cursor_move_1ms_target", |b| {
        b.iter_batched(
            || NavigationSystem::with_high_performance(),
            |mut nav| {
                let _ = nav.navigate(black_box(short_text), NavigationAction::MoveCharForward);
            },
            BatchSize::SmallInput,
        )
    });

    // 長い行での制限時間検証（5ms未満）
    let long_line = "x".repeat(1000);
    group.bench_function("long_line_5ms_target", |b| {
        b.iter_batched(
            || NavigationSystem::with_high_performance(),
            |mut nav| {
                let _ = nav.navigate(black_box(&long_line), NavigationAction::MoveCharForward);
            },
            BatchSize::SmallInput,
        )
    });

    // 超長い行での制限時間検証（10ms未満）
    let very_long_line = "x".repeat(10000);
    group.bench_function("very_long_line_10ms_target", |b| {
        b.iter_batched(
            || NavigationSystem::with_high_performance(),
            |mut nav| {
                let _ = nav.navigate(
                    black_box(&very_long_line),
                    NavigationAction::MoveCharForward,
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cursor_movement,
    bench_high_performance_navigation,
    bench_line_navigation,
    bench_buffer_navigation,
    bench_tab_navigation,
    bench_performance_targets
);
criterion_main!(benches);
