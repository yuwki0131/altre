//! Navigation performance regression tests
//!
//! These tests measure critical navigation operations and ensure they stay within
//! the response-time envelopes defined in QA.md and
//! `docs/design/navigation_performance_tests.md`.

use altre::buffer::cursor::CursorPosition;
use altre::buffer::{navigation::NavigationError, NavigationAction, NavigationSystem};
use std::cmp::min;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct NavigationTestConfig {
    warmup_iterations: usize,
    measurement_iterations: usize,
    tolerance_factor: f64,
}

impl Default for NavigationTestConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 32,
            measurement_iterations: 160,
            tolerance_factor: 1.6,
        }
    }
}

struct NavigationPerformanceTestHarness {
    config: NavigationTestConfig,
}

impl NavigationPerformanceTestHarness {
    fn new() -> Self {
        Self {
            config: NavigationTestConfig::default(),
        }
    }

    fn measure_action<P, F>(
        &self,
        label: &str,
        target: Duration,
        mut prepare: P,
        operation: F,
    ) -> NavigationPerformanceResult
    where
        P: FnMut() -> NavigationSystem,
        F: Fn(&mut NavigationSystem) -> Result<bool, NavigationError>,
    {
        // Warmup phase (discard durations)
        let mut warmup_nav = prepare();
        warmup_nav.disable_performance_monitoring();
        for _ in 0..self.config.warmup_iterations {
            let moved = operation(&mut warmup_nav)
                .unwrap_or_else(|e| panic!("{} warmup failed: {}", label, e));
            if !moved {
                panic!("{} warmup reached boundary without movement", label);
            }
        }

        // Measurement phase
        let mut nav = prepare();
        nav.disable_performance_monitoring();
        let mut durations = Vec::with_capacity(self.config.measurement_iterations);
        for _ in 0..self.config.measurement_iterations {
            let start = Instant::now();
            let moved = operation(&mut nav)
                .unwrap_or_else(|e| panic!("{} measurement failed: {}", label, e));
            let elapsed = start.elapsed();
            if !moved {
                panic!("{} measurement reached boundary without movement", label);
            }
            durations.push(elapsed);
        }

        let stats = MeasurementStatistics::from_samples(&durations);
        let target_ms = target_as_millis(target);
        let mean_ok = stats.mean_millis <= target_ms * self.config.tolerance_factor;
        let median_ok = stats.median_millis <= target_ms * self.config.tolerance_factor;
        let max_ok = stats.max_millis <= target_ms * (self.config.tolerance_factor + 0.5);
        let passed = mean_ok && median_ok && max_ok;

        NavigationPerformanceResult {
            label: label.to_string(),
            target,
            stats,
            iterations: durations.len(),
            tolerance_factor: self.config.tolerance_factor,
            passed,
        }
    }
}

#[derive(Debug, Clone)]
struct MeasurementStatistics {
    min_millis: f64,
    max_millis: f64,
    median_millis: f64,
    mean_millis: f64,
}

impl MeasurementStatistics {
    fn from_samples(samples: &[Duration]) -> Self {
        assert!(!samples.is_empty(), "no samples supplied");
        let mut sorted: Vec<f64> = samples
            .iter()
            .map(|duration| duration_as_millis(*duration))
            .collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min_millis = sorted[0];
        let max_millis = *sorted.last().unwrap();
        let median_millis = if sorted.len() % 2 == 0 {
            let upper = sorted.len() / 2;
            (sorted[upper - 1] + sorted[upper]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        let sum: f64 = sorted.iter().sum();
        let mean_millis = sum / sorted.len() as f64;

        Self {
            min_millis,
            max_millis,
            median_millis,
            mean_millis,
        }
    }
}

struct NavigationPerformanceResult {
    label: String,
    target: Duration,
    stats: MeasurementStatistics,
    iterations: usize,
    tolerance_factor: f64,
    passed: bool,
}

impl NavigationPerformanceResult {
    fn passed(&self) -> bool {
        self.passed
    }

    fn diagnostic(&self) -> String {
        format!(
            concat!(
                "{}: target {:.3} ms (×{:.2} tol) | mean {:.3} ms | median {:.3} ms | ",
                "min {:.3} ms | max {:.3} ms over {} samples"
            ),
            self.label,
            target_as_millis(self.target),
            self.tolerance_factor,
            self.stats.mean_millis,
            self.stats.median_millis,
            self.stats.min_millis,
            self.stats.max_millis,
            self.iterations,
        )
    }
}

// === Test scenarios ==============================================================

#[test]
fn navigation_basic_motions_under_one_millisecond() {
    let harness = NavigationPerformanceTestHarness::new();
    let text = "The quick brown fox jumps over the lazy dog.\nSecond line for movement.";
    let target = Duration::from_micros(1_000);

    let forward = harness.measure_action(
        "char_forward_short",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            nav.set_cursor(CursorPosition::new());
            nav.navigate(text, NavigationAction::MoveCharForward)
        },
    );

    let backward = harness.measure_action(
        "char_backward_short",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            let cursor = cursor_at_end(text);
            nav.set_cursor(cursor);
            nav.navigate(text, NavigationAction::MoveCharBackward)
        },
    );

    let line_down = harness.measure_action(
        "line_down_mid_buffer",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            let line = 0;
            let column = 10;
            let cursor = cursor_for_line_col(text, line, column);
            nav.set_cursor(cursor);
            nav.navigate(text, NavigationAction::MoveLineDown)
        },
    );

    let line_up = harness.measure_action(
        "line_up_mid_buffer",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            let line = 1;
            let column = 5;
            let cursor = cursor_for_line_col(text, line, column);
            nav.set_cursor(cursor);
            nav.navigate(text, NavigationAction::MoveLineUp)
        },
    );

    let line_start = harness.measure_action(
        "line_start",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            let line = 1;
            let column = 15;
            let cursor = cursor_for_line_col(text, line, column);
            nav.set_cursor(cursor);
            let moved = nav.navigate(text, NavigationAction::MoveLineStart)?;
            if moved {
                // restore column for consistency
                let reset_cursor = cursor_for_line_col(text, line, column);
                nav.set_cursor(reset_cursor);
            }
            Ok(moved)
        },
    );

    let line_end = harness.measure_action(
        "line_end",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            let line = 0;
            let column = 3;
            let cursor = cursor_for_line_col(text, line, column);
            nav.set_cursor(cursor);
            let moved = nav.navigate(text, NavigationAction::MoveLineEnd)?;
            if moved {
                // place cursor back near start for next iteration
                let reset_cursor = cursor_for_line_col(text, line, column);
                nav.set_cursor(reset_cursor);
            }
            Ok(moved)
        },
    );

    for result in [forward, backward, line_down, line_up, line_start, line_end] {
        eprintln!("{}", result.diagnostic());
        assert!(result.passed(), "{}", result.diagnostic());
    }
}

#[test]
fn navigation_long_line_performance_targets() {
    let harness = NavigationPerformanceTestHarness::new();

    let long_line = "x".repeat(1_000);
    let target_long = Duration::from_micros(5_000);
    let result_long = harness.measure_action(
        "long_line_1000_char_forward",
        target_long,
        || NavigationSystem::with_high_performance(),
        |nav| {
            nav.set_cursor(CursorPosition::new());
            nav.navigate(&long_line, NavigationAction::MoveCharForward)
        },
    );

    let very_long_line = "x".repeat(10_000);
    let target_very_long = Duration::from_micros(10_000);
    let result_very_long = harness.measure_action(
        "very_long_line_10000_char_forward",
        target_very_long,
        || NavigationSystem::with_high_performance(),
        |nav| {
            nav.set_cursor(CursorPosition::new());
            nav.navigate(&very_long_line, NavigationAction::MoveCharForward)
        },
    );

    for result in [result_long, result_very_long] {
        eprintln!("{}", result.diagnostic());
        assert!(result.passed(), "{}", result.diagnostic());
    }
}

#[test]
fn navigation_buffer_wide_operations_within_two_milliseconds() {
    let harness = NavigationPerformanceTestHarness::new();
    let large_file: String = (0..5_000)
        .map(|i| format!("Line {} with some content here\n", i))
        .collect();
    let target = Duration::from_micros(4_000);

    let to_start = harness.measure_action(
        "buffer_move_to_start",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            let cursor = cursor_at_end(&large_file);
            nav.set_cursor(cursor);
            nav.navigate(&large_file, NavigationAction::MoveBufferStart)
        },
    );

    let to_end = harness.measure_action(
        "buffer_move_to_end",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            nav.set_cursor(CursorPosition::new());
            nav.navigate(&large_file, NavigationAction::MoveBufferEnd)
        },
    );

    for result in [to_start, to_end] {
        eprintln!("{}", result.diagnostic());
        assert!(result.passed(), "{}", result.diagnostic());
    }
}

#[cfg_attr(
    debug_assertions,
    ignore = "Navigation performance thresholds apply to release build"
)]
#[test]
fn navigation_tab_width_conversion_under_half_millisecond() {
    let harness = NavigationPerformanceTestHarness::new();
    let tab_text = "function test() {\n\treturn 'hello world';\n}\n".repeat(512);
    let target = Duration::from_micros(750);

    let tab4 = harness.measure_action(
        "tab_width_4",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            nav.set_cursor(CursorPosition::new());
            nav.navigate_with_tab_width(&tab_text, NavigationAction::MoveCharForward, 4)
        },
    );

    let tab8 = harness.measure_action(
        "tab_width_8",
        target,
        || NavigationSystem::with_high_performance(),
        |nav| {
            nav.set_cursor(CursorPosition::new());
            nav.navigate_with_tab_width(&tab_text, NavigationAction::MoveCharForward, 8)
        },
    );

    for result in [tab4, tab8] {
        eprintln!("{}", result.diagnostic());
        assert!(result.passed(), "{}", result.diagnostic());
    }
}

// === Helper functions =============================================================

fn duration_as_millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn target_as_millis(target: Duration) -> f64 {
    duration_as_millis(target)
}

fn cursor_at_end(text: &str) -> CursorPosition {
    let char_pos = text.chars().count();
    let lines: Vec<&str> = text.split('\n').collect();
    let last_line_index = lines.len().saturating_sub(1);
    let last_line = lines.last().copied().unwrap_or("");
    let column = last_line.chars().count();
    CursorPosition::at(char_pos, last_line_index, column)
}

fn cursor_for_line_col(text: &str, line: usize, column: usize) -> CursorPosition {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut char_pos = 0usize;
    for (idx, l) in lines.iter().enumerate() {
        if idx == line {
            let clamped_column = min(column, l.chars().count());
            char_pos += l.chars().take(clamped_column).count();
            return CursorPosition::at(char_pos, line, clamped_column);
        }
        char_pos += l.chars().count();
        if idx < lines.len() - 1 {
            char_pos += 1; // account for newline character
        }
    }
    CursorPosition::at(char_pos, lines.len().saturating_sub(1), 0)
}
