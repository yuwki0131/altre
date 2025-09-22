# ナビゲーションパフォーマンステスト仕様書

## 概要

本文書は、Altreテキストエディタのナビゲーション機能における性能要件とテスト仕様を定義する。QA.mdで定められた「カーソル移動 < 1ms」要件を含む、全ナビゲーション操作の性能目標と測定方法を明確化する。

## 性能要件（QA回答に基づく）

### 基本性能目標

| 操作カテゴリ | 操作 | 目標応答時間 | 測定条件 | QA回答 |
|-------------|------|-------------|----------|---------|
| **基本移動** | カーソル移動全般 | < 1ms | 任意のファイルサイズ | Q2回答 |
| **文字移動** | 前後移動（C-f, C-b, ←, →） | < 1ms | 通常の行長 | Q2回答 |
| **行移動** | 上下移動（C-p, C-n, ↑, ↓） | < 1ms | 通常の行長 | Q2回答 |
| **行内移動** | 行頭・行末移動（C-a, C-e） | < 1ms | 通常の行長 | Q2回答 |
| **バッファ移動** | ファイル先頭・末尾移動 | < 2ms | 大きなファイル | 推定 |

### 長い行でのパフォーマンス（QA Q22回答）

| 行長 | 目標応答時間 | 適用操作 | 制限理由 |
|------|-------------|----------|----------|
| < 1,000文字 | < 1ms | 全ナビゲーション操作 | 通常目標維持 |
| 1,000-10,000文字 | < 5ms | 全ナビゲーション操作 | 段階的制限許容 |
| > 10,000文字 | < 10ms | 全ナビゲーション操作 | 性能劣化許容 |

### Tab幅計算パフォーマンス（QA Q21回答）

| 操作 | 目標応答時間 | Tab幅設定 |
|------|-------------|----------|
| 表示列計算 | < 0.5ms | 4スペース固定 |
| 論理→表示列変換 | < 0.5ms | 4スペース固定 |
| 表示→論理列変換 | < 0.5ms | 4スペース固定 |

## テストフレームワーク

### ナビゲーション専用測定システム

```rust
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::buffer::navigation::{NavigationSystem, NavigationAction, Position};

/// ナビゲーション性能測定結果
#[derive(Debug, Clone)]
pub struct NavigationPerformanceResult {
    /// 測定対象操作
    pub action: NavigationAction,
    /// 実行時間
    pub duration: Duration,
    /// 目標時間
    pub target_duration: Duration,
    /// テストケース情報
    pub test_case: TestCaseInfo,
    /// 成功/失敗
    pub passed: bool,
    /// 追加メタデータ
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct TestCaseInfo {
    /// テストケース名
    pub name: String,
    /// ファイルサイズ（文字数）
    pub file_size: usize,
    /// 最大行長
    pub max_line_length: usize,
    /// 総行数
    pub total_lines: usize,
}

/// ナビゲーション性能テストハーネス
pub struct NavigationPerformanceTestHarness {
    /// 測定結果
    results: Vec<NavigationPerformanceResult>,
    /// 測定設定
    config: NavigationTestConfig,
    /// ナビゲーションシステム
    nav_system: NavigationSystem,
}

#[derive(Debug, Clone)]
pub struct NavigationTestConfig {
    /// ウォームアップ回数
    pub warmup_iterations: usize,
    /// 測定回数
    pub measurement_iterations: usize,
    /// 詳細ログを有効にするか
    pub verbose_logging: bool,
    /// パフォーマンス制約の厳密度
    pub strict_constraints: bool,
}

impl NavigationPerformanceTestHarness {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            config: NavigationTestConfig {
                warmup_iterations: 20,
                measurement_iterations: 100,
                verbose_logging: false,
                strict_constraints: true,
            },
            nav_system: NavigationSystem::new(),
        }
    }

    /// ナビゲーション操作のパフォーマンス測定
    pub fn measure_navigation_operation(
        &mut self,
        action: NavigationAction,
        text: &str,
        target_duration: Duration,
        test_case_name: &str,
    ) -> NavigationPerformanceResult {
        let test_case = TestCaseInfo {
            name: test_case_name.to_string(),
            file_size: text.chars().count(),
            max_line_length: text.lines().map(|l| l.chars().count()).max().unwrap_or(0),
            total_lines: text.lines().count(),
        };

        // ウォームアップ
        for _ in 0..self.config.warmup_iterations {
            let _ = self.nav_system.navigate(text, action);
            self.nav_system = NavigationSystem::new(); // リセット
        }

        // 実際の測定
        let mut durations = Vec::new();
        for _ in 0..self.config.measurement_iterations {
            self.nav_system = NavigationSystem::new(); // クリーンな状態で測定

            let start = Instant::now();
            let _ = self.nav_system.navigate(text, action);
            durations.push(start.elapsed());
        }

        // 統計計算
        let median_duration = self.calculate_median(&durations);
        let passed = median_duration <= target_duration;

        let mut metadata = HashMap::new();
        metadata.insert("min_duration".to_string(), format!("{:?}", durations.iter().min().unwrap()));
        metadata.insert("max_duration".to_string(), format!("{:?}", durations.iter().max().unwrap()));
        metadata.insert("avg_duration".to_string(), format!("{:?}", Duration::from_nanos(
            durations.iter().map(|d| d.as_nanos()).sum::<u128>() / durations.len() as u128
        )));

        let result = NavigationPerformanceResult {
            action,
            duration: median_duration,
            target_duration,
            test_case,
            passed,
            metadata,
        };

        self.results.push(result.clone());
        result
    }

    /// 中央値を計算
    fn calculate_median(&self, durations: &[Duration]) -> Duration {
        let mut sorted = durations.to_vec();
        sorted.sort();
        let mid = sorted.len() / 2;
        sorted[mid]
    }

    /// テスト結果をレポート
    pub fn generate_report(&self) -> NavigationTestReport {
        NavigationTestReport::new(&self.results)
    }
}
```

## テストケース仕様

### 1. 基本ナビゲーション性能テスト

```rust
#[cfg(test)]
mod basic_navigation_tests {
    use super::*;

    #[test]
    fn test_character_movement_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "Hello, World! ".repeat(100); // 1400文字程度

        // 右移動（C-f, →）
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(1),
            "char_forward_medium_text"
        );
        assert!(result.passed, "Character forward movement failed: {:?}", result.duration);

        // 左移動（C-b, ←）
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharBackward,
            &text,
            Duration::from_millis(1),
            "char_backward_medium_text"
        );
        assert!(result.passed, "Character backward movement failed: {:?}", result.duration);
    }

    #[test]
    fn test_line_movement_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let lines = (0..100).map(|i| format!("Line {} with some content", i)).collect::<Vec<_>>();
        let text = lines.join("\n");

        // 下移動（C-n, ↓）
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineDown,
            &text,
            Duration::from_millis(1),
            "line_down_multiline"
        );
        assert!(result.passed, "Line down movement failed: {:?}", result.duration);

        // 上移動（C-p, ↑）
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineUp,
            &text,
            Duration::from_millis(1),
            "line_up_multiline"
        );
        assert!(result.passed, "Line up movement failed: {:?}", result.duration);
    }

    #[test]
    fn test_line_boundary_movement_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "Short\nThis is a much longer line with many characters\nShort";

        // 行頭移動（C-a）
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineStart,
            &text,
            Duration::from_millis(1),
            "line_start_movement"
        );
        assert!(result.passed, "Line start movement failed: {:?}", result.duration);

        // 行末移動（C-e）
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineEnd,
            &text,
            Duration::from_millis(1),
            "line_end_movement"
        );
        assert!(result.passed, "Line end movement failed: {:?}", result.duration);
    }

    #[test]
    fn test_buffer_boundary_movement_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "line\n".repeat(1000); // 5000文字程度

        // バッファ先頭移動
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveBufferStart,
            &text,
            Duration::from_millis(2),
            "buffer_start_movement"
        );
        assert!(result.passed, "Buffer start movement failed: {:?}", result.duration);

        // バッファ末尾移動
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveBufferEnd,
            &text,
            Duration::from_millis(2),
            "buffer_end_movement"
        );
        assert!(result.passed, "Buffer end movement failed: {:?}", result.duration);
    }
}
```

### 2. 長い行性能テスト（QA Q22対応）

```rust
#[cfg(test)]
mod long_line_performance_tests {
    use super::*;

    #[test]
    fn test_short_line_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "a".repeat(500); // 500文字の行

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(1), // 通常目標維持
            "short_line_500_chars"
        );
        assert!(result.passed, "Short line navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_medium_line_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "a".repeat(5000); // 5000文字の行

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(5), // 段階的制限許容
            "medium_line_5000_chars"
        );
        assert!(result.passed, "Medium line navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_long_line_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "a".repeat(50000); // 50000文字の行

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(10), // 性能劣化許容
            "long_line_50000_chars"
        );
        assert!(result.passed, "Long line navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_line_movement_with_varying_lengths() {
        let mut harness = NavigationPerformanceTestHarness::new();

        // 様々な長さの行を含むテキスト
        let mut lines = Vec::new();
        lines.push("short".to_string());
        lines.push("a".repeat(1000)); // 1000文字の行
        lines.push("medium line".to_string());
        lines.push("a".repeat(10000)); // 10000文字の行
        lines.push("another short".to_string());

        let text = lines.join("\n");

        // 行移動のパフォーマンス
        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineDown,
            &text,
            Duration::from_millis(5), // 最大行長に基づく制限
            "mixed_line_lengths"
        );
        assert!(result.passed, "Mixed line length navigation failed: {:?}", result.duration);
    }
}
```

### 3. UTF-8文字性能テスト

```rust
#[cfg(test)]
mod utf8_performance_tests {
    use super::*;

    #[test]
    fn test_ascii_character_navigation() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "abcdefghij".repeat(100); // ASCII文字のみ

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(1),
            "ascii_only_navigation"
        );
        assert!(result.passed, "ASCII navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_japanese_character_navigation() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "こんにちは世界".repeat(100); // 日本語文字

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(1),
            "japanese_navigation"
        );
        assert!(result.passed, "Japanese navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_emoji_character_navigation() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "🌟🚀🎉🎈🌈".repeat(100); // 絵文字

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(1),
            "emoji_navigation"
        );
        assert!(result.passed, "Emoji navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_mixed_unicode_navigation() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "Hello 🌟 こんにちは 世界! ".repeat(100); // 混合Unicode

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(1),
            "mixed_unicode_navigation"
        );
        assert!(result.passed, "Mixed Unicode navigation failed: {:?}", result.duration);
    }
}
```

### 4. Tab幅計算性能テスト（QA Q21対応）

```rust
#[cfg(test)]
mod tab_performance_tests {
    use super::*;
    use crate::buffer::navigation::{Position, PositionCalculator};

    #[test]
    fn test_tab_width_calculation_performance() {
        let text_with_tabs = "a\tb\tc\td\te\tf\tg\th\ti\tj".repeat(100);

        let start = Instant::now();
        for i in 0..100 {
            let logical_col = i * 10;
            let _ = Position::logical_to_visual_column(logical_col, &text_with_tabs, 4);
        }
        let duration = start.elapsed();

        assert!(duration.as_millis() < 50, "Tab calculation too slow: {:?}", duration);
    }

    #[test]
    fn test_visual_to_logical_conversion_performance() {
        let text_with_tabs = "a\tb\tc\td\te".repeat(200);

        let start = Instant::now();
        for visual_col in (0..1000).step_by(10) {
            let _ = Position::visual_to_logical_column(visual_col, &text_with_tabs, 4);
        }
        let duration = start.elapsed();

        assert!(duration.as_millis() < 50, "Visual to logical conversion too slow: {:?}", duration);
    }

    #[test]
    fn test_mixed_tab_space_performance() {
        let complex_line = "func\t\tname(\tparam1,\n\t\t\tparam2\t)\t{".repeat(50);

        let start = Instant::now();
        for logical_col in (0..complex_line.chars().count()).step_by(5) {
            let _ = Position::logical_to_visual_column(logical_col, &complex_line, 4);
        }
        let duration = start.elapsed();

        assert!(duration.as_millis() < 25, "Mixed tab/space calculation too slow: {:?}", duration);
    }
}
```

### 5. 位置計算性能テスト

```rust
#[cfg(test)]
mod position_calculation_tests {
    use super::*;
    use crate::buffer::navigation::PositionCalculator;

    #[test]
    fn test_char_to_line_col_performance() {
        let mut calc = PositionCalculator::new();
        let text = "line\n".repeat(1000); // 1000行

        let start = Instant::now();
        for i in (0..text.chars().count()).step_by(100) {
            let _ = calc.char_pos_to_line_col(&text, i).unwrap();
        }
        let duration = start.elapsed();

        assert!(duration.as_millis() < 10, "Char to line/col conversion too slow: {:?}", duration);
    }

    #[test]
    fn test_line_col_to_char_performance() {
        let mut calc = PositionCalculator::new();
        let text = "line with some content\n".repeat(1000);

        let start = Instant::now();
        for line in (0..1000).step_by(10) {
            for col in [0, 5, 10, 15, 20] {
                let _ = calc.line_col_to_char_pos(&text, line, col).unwrap();
            }
        }
        let duration = start.elapsed();

        assert!(duration.as_millis() < 10, "Line/col to char conversion too slow: {:?}", duration);
    }

    #[test]
    fn test_cache_rebuild_performance() {
        let mut calc = PositionCalculator::new();
        let large_text = "a very long line with many characters\n".repeat(5000);

        let start = Instant::now();
        calc.position_engine().invalidate_cache();
        let _ = calc.char_pos_to_line_col(&large_text, 1000).unwrap(); // キャッシュ再構築をトリガー
        let duration = start.elapsed();

        assert!(duration.as_millis() < 100, "Cache rebuild too slow: {:?}", duration);
    }

    #[test]
    fn test_position_calculation_with_long_lines() {
        let mut calc = PositionCalculator::new();

        // 極端に長い行を含むテキスト
        let mut lines = Vec::new();
        lines.push("short line".to_string());
        lines.push("a".repeat(20000)); // 20000文字の行
        lines.push("another short line".to_string());
        let text = lines.join("\n");

        let start = Instant::now();

        // 長い行の中間での位置計算
        let long_line_middle = 10000 + 11; // "short line\n" + 10000文字
        let _ = calc.char_pos_to_line_col(&text, long_line_middle).unwrap();

        let duration = start.elapsed();

        // QA Q22: 長い行では10ms許容
        assert!(duration.as_millis() < 10, "Long line position calculation too slow: {:?}", duration);
    }
}
```

## スケーラビリティテスト

### 大きなファイルでの性能テスト

```rust
#[cfg(test)]
mod scalability_tests {
    use super::*;

    #[test]
    fn test_small_file_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "line\n".repeat(100); // ~500文字

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineDown,
            &text,
            Duration::from_millis(1),
            "small_file_100_lines"
        );
        assert!(result.passed, "Small file navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_medium_file_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "line with some content\n".repeat(1000); // ~23KB

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineDown,
            &text,
            Duration::from_millis(1),
            "medium_file_1000_lines"
        );
        assert!(result.passed, "Medium file navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_large_file_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "line with some content here\n".repeat(10000); // ~280KB

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveLineDown,
            &text,
            Duration::from_millis(2), // 大きなファイルでは少し緩和
            "large_file_10000_lines"
        );
        assert!(result.passed, "Large file navigation failed: {:?}", result.duration);
    }

    #[test]
    fn test_very_large_file_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "line\n".repeat(50000); // ~250KB

        let result = harness.measure_navigation_operation(
            NavigationAction::MoveCharForward,
            &text,
            Duration::from_millis(5), // 非常に大きなファイルではさらに緩和
            "very_large_file_50000_lines"
        );
        assert!(result.passed, "Very large file navigation failed: {:?}", result.duration);
    }
}
```

## ストレステスト

### 極限条件での性能テスト

```rust
#[cfg(test)]
mod stress_tests {
    use super::*;

    #[test]
    fn test_rapid_navigation_sequence() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "line\n".repeat(1000);

        let start = Instant::now();

        // 連続的なナビゲーション操作
        let actions = [
            NavigationAction::MoveCharForward,
            NavigationAction::MoveLineDown,
            NavigationAction::MoveCharBackward,
            NavigationAction::MoveLineUp,
            NavigationAction::MoveLineEnd,
            NavigationAction::MoveLineStart,
        ];

        for _ in 0..100 {
            for &action in &actions {
                harness.nav_system.navigate(&text, action).unwrap();
            }
        }

        let duration = start.elapsed();
        let operations_count = 100 * actions.len();
        let avg_per_operation = duration / operations_count as u32;

        assert!(avg_per_operation.as_millis() < 1,
               "Rapid navigation sequence too slow: avg {:?} per operation", avg_per_operation);
    }

    #[test]
    fn test_boundary_condition_performance() {
        let mut harness = NavigationPerformanceTestHarness::new();
        let text = "a".repeat(10000);

        let start = Instant::now();

        // 境界条件での操作
        for _ in 0..50 {
            // ファイル先頭での左移動試行
            harness.nav_system = NavigationSystem::new(); // 先頭に戻す
            let _ = harness.nav_system.navigate(&text, NavigationAction::MoveCharBackward);

            // ファイル末尾に移動してから右移動試行
            let _ = harness.nav_system.navigate(&text, NavigationAction::MoveBufferEnd);
            let _ = harness.nav_system.navigate(&text, NavigationAction::MoveCharForward);
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 50, "Boundary condition handling too slow: {:?}", duration);
    }

    #[test]
    fn test_alternating_long_short_lines() {
        let mut harness = NavigationPerformanceTestHarness::new();

        let mut lines = Vec::new();
        for i in 0..1000 {
            if i % 2 == 0 {
                lines.push("short".to_string());
            } else {
                lines.push("a".repeat(5000)); // 長い行
            }
        }
        let text = lines.join("\n");

        let start = Instant::now();

        // 長い行と短い行を交互に移動
        for _ in 0..100 {
            let _ = harness.nav_system.navigate(&text, NavigationAction::MoveLineDown);
        }

        let duration = start.elapsed();

        // 長い行が混在するため、段階的制限を適用
        assert!(duration.as_millis() < 500, "Alternating line navigation too slow: {:?}", duration);
    }
}
```

## パフォーマンス回帰テスト

### CI/CD統合用テスト

```rust
/// CI/CD環境用の軽量パフォーマンステスト
pub fn run_navigation_performance_regression_tests() -> NavigationTestReport {
    let mut harness = NavigationPerformanceTestHarness::new();
    harness.config.measurement_iterations = 20; // CI環境では少なめに

    let mut all_passed = true;

    // 重要なナビゲーション操作のみテスト
    let test_cases = vec![
        ("basic_char_forward", "a".repeat(1000), NavigationAction::MoveCharForward, Duration::from_millis(1)),
        ("basic_line_down", "line\n".repeat(100), NavigationAction::MoveLineDown, Duration::from_millis(1)),
        ("long_line_navigation", "a".repeat(10000), NavigationAction::MoveCharForward, Duration::from_millis(10)),
        ("large_file_navigation", "line\n".repeat(5000), NavigationAction::MoveLineDown, Duration::from_millis(2)),
    ];

    for (test_name, text, action, target) in test_cases {
        let result = harness.measure_navigation_operation(action, &text, target, test_name);
        if !result.passed {
            all_passed = false;
            eprintln!("REGRESSION: {} failed with {:?} (target: {:?})", test_name, result.duration, target);
        }
    }

    let report = harness.generate_report();

    if !all_passed {
        panic!("Navigation performance regression detected!");
    }

    report
}

/// ベンチマーク結果の比較
pub fn compare_with_baseline(current: &NavigationTestReport, baseline_file: &str) -> bool {
    // ベースライン結果をファイルから読み込み
    // 現在の結果と比較して回帰を検出
    // 実装は簡略化
    true
}
```

## レポート生成

### テスト結果レポート

```rust
/// ナビゲーションテスト結果レポート
pub struct NavigationTestReport {
    /// 実行されたテスト結果
    pub results: Vec<NavigationPerformanceResult>,
    /// 全体の成功率
    pub success_rate: f64,
    /// 操作別統計
    pub action_statistics: HashMap<NavigationAction, ActionStatistics>,
    /// ファイルサイズ別統計
    pub size_statistics: HashMap<String, SizeStatistics>,
}

#[derive(Debug, Clone)]
pub struct ActionStatistics {
    pub action: NavigationAction,
    pub test_count: usize,
    pub success_count: usize,
    pub average_duration: Duration,
    pub median_duration: Duration,
    pub worst_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct SizeStatistics {
    pub size_category: String,
    pub test_count: usize,
    pub success_count: usize,
    pub average_duration: Duration,
}

impl NavigationTestReport {
    pub fn new(results: &[NavigationPerformanceResult]) -> Self {
        let success_count = results.iter().filter(|r| r.passed).count();
        let success_rate = success_count as f64 / results.len() as f64;

        let mut action_stats = HashMap::new();
        let mut size_stats = HashMap::new();

        // 操作別統計の計算
        for action in [
            NavigationAction::MoveCharForward,
            NavigationAction::MoveCharBackward,
            NavigationAction::MoveLineUp,
            NavigationAction::MoveLineDown,
            NavigationAction::MoveLineStart,
            NavigationAction::MoveLineEnd,
            NavigationAction::MoveBufferStart,
            NavigationAction::MoveBufferEnd,
        ] {
            let action_results: Vec<_> = results.iter().filter(|r| r.action == action).collect();
            if !action_results.is_empty() {
                action_stats.insert(action, Self::calculate_action_statistics(action, &action_results));
            }
        }

        // サイズ別統計の計算
        for size_category in ["small", "medium", "large", "very_large"] {
            let size_results: Vec<_> = results.iter().filter(|r|
                Self::categorize_file_size(r.test_case.file_size) == size_category
            ).collect();
            if !size_results.is_empty() {
                size_stats.insert(size_category.to_string(), Self::calculate_size_statistics(size_category, &size_results));
            }
        }

        Self {
            results: results.to_vec(),
            success_rate,
            action_statistics: action_stats,
            size_statistics: size_stats,
        }
    }

    fn calculate_action_statistics(action: NavigationAction, results: &[&NavigationPerformanceResult]) -> ActionStatistics {
        let test_count = results.len();
        let success_count = results.iter().filter(|r| r.passed).count();

        let durations: Vec<_> = results.iter().map(|r| r.duration).collect();
        let average_duration = Duration::from_nanos(
            durations.iter().map(|d| d.as_nanos()).sum::<u128>() / durations.len() as u128
        );

        let mut sorted_durations = durations;
        sorted_durations.sort();
        let median_duration = sorted_durations[sorted_durations.len() / 2];
        let worst_duration = sorted_durations.last().copied().unwrap_or_default();

        ActionStatistics {
            action,
            test_count,
            success_count,
            average_duration,
            median_duration,
            worst_duration,
        }
    }

    fn calculate_size_statistics(size_category: &str, results: &[&NavigationPerformanceResult]) -> SizeStatistics {
        let test_count = results.len();
        let success_count = results.iter().filter(|r| r.passed).count();

        let average_duration = Duration::from_nanos(
            results.iter().map(|r| r.duration.as_nanos()).sum::<u128>() / results.len() as u128
        );

        SizeStatistics {
            size_category: size_category.to_string(),
            test_count,
            success_count,
            average_duration,
        }
    }

    fn categorize_file_size(file_size: usize) -> &'static str {
        match file_size {
            0..=1000 => "small",
            1001..=10000 => "medium",
            10001..=100000 => "large",
            _ => "very_large",
        }
    }

    /// HTMLレポートを生成
    pub fn generate_html_report(&self) -> String {
        format!(
            r#"
            <html>
            <head><title>Navigation Performance Test Report</title></head>
            <body>
                <h1>Navigation Performance Test Results</h1>
                <h2>Summary</h2>
                <p>Success Rate: {:.1}%</p>
                <p>Total Tests: {}</p>

                <h2>Action Statistics</h2>
                <table border="1">
                    <tr><th>Action</th><th>Tests</th><th>Success Rate</th><th>Avg Duration</th><th>Median Duration</th><th>Worst Duration</th></tr>
                    {}
                </table>

                <h2>File Size Statistics</h2>
                <table border="1">
                    <tr><th>Size Category</th><th>Tests</th><th>Success Rate</th><th>Avg Duration</th></tr>
                    {}
                </table>
            </body>
            </html>
            "#,
            self.success_rate * 100.0,
            self.results.len(),
            self.generate_action_rows(),
            self.generate_size_rows()
        )
    }

    fn generate_action_rows(&self) -> String {
        self.action_statistics
            .values()
            .map(|stats| {
                format!(
                    "<tr><td>{:?}</td><td>{}</td><td>{:.1}%</td><td>{:.2}ms</td><td>{:.2}ms</td><td>{:.2}ms</td></tr>",
                    stats.action,
                    stats.test_count,
                    (stats.success_count as f64 / stats.test_count as f64) * 100.0,
                    stats.average_duration.as_secs_f64() * 1000.0,
                    stats.median_duration.as_secs_f64() * 1000.0,
                    stats.worst_duration.as_secs_f64() * 1000.0
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn generate_size_rows(&self) -> String {
        self.size_statistics
            .values()
            .map(|stats| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{:.1}%</td><td>{:.2}ms</td></tr>",
                    stats.size_category,
                    stats.test_count,
                    (stats.success_count as f64 / stats.test_count as f64) * 100.0,
                    stats.average_duration.as_secs_f64() * 1000.0
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// コンソール出力用レポート
    pub fn print_console_report(&self) {
        println!("\n=== Navigation Performance Test Report ===");
        println!("Success Rate: {:.1}%", self.success_rate * 100.0);
        println!("Total Tests: {}", self.results.len());

        println!("\nAction Statistics:");
        for (action, stats) in &self.action_statistics {
            println!(
                "  {:?}: {}/{} passed ({:.1}%), avg: {:.2}ms, worst: {:.2}ms",
                action,
                stats.success_count,
                stats.test_count,
                (stats.success_count as f64 / stats.test_count as f64) * 100.0,
                stats.average_duration.as_secs_f64() * 1000.0,
                stats.worst_duration.as_secs_f64() * 1000.0
            );
        }

        println!("\nFile Size Statistics:");
        for (size, stats) in &self.size_statistics {
            println!(
                "  {}: {}/{} passed ({:.1}%), avg: {:.2}ms",
                size,
                stats.success_count,
                stats.test_count,
                (stats.success_count as f64 / stats.test_count as f64) * 100.0,
                stats.average_duration.as_secs_f64() * 1000.0
            );
        }
        println!("==========================================\n");
    }
}
```

## 実行方法

### テスト実行コマンド

```bash
# 基本ナビゲーション性能テスト
cargo test --test navigation_performance_tests --release

# 詳細レポート付き実行
cargo test --test navigation_performance_tests --release -- --nocapture

# 長い行性能テスト
cargo test long_line_performance_tests --release

# ストレステスト
cargo test stress_tests --release

# CI用回帰テスト
cargo test --test navigation_regression_tests --release

# Criterionベンチマーク
cargo bench navigation_benchmark
```

### 継続的監視

```bash
# パフォーマンス監視スクリプト
#!/bin/bash
# performance_monitor.sh

echo "Running navigation performance tests..."
cargo test --test navigation_performance_tests --release > perf_results.txt 2>&1

if [ $? -eq 0 ]; then
    echo "All navigation performance tests passed"
else
    echo "ALERT: Navigation performance regression detected!"
    cat perf_results.txt
    exit 1
fi
```

## まとめ

この仕様により、Altreエディタのナビゲーション機能が常に高いパフォーマンスを維持し、QA.mdで定められた性能要件（カーソル移動 < 1ms、長い行での段階的制限許容）を確実に満たすことを保証する。また、継続的な監視により性能回帰を早期検出し、ユーザーに一貫して快適な編集体験を提供できる。