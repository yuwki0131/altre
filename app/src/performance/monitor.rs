//! パフォーマンス監視システム
//!
//! リアルタイムの性能測定と監視機能を提供

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 監視対象の操作種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    CursorMove,
    Insert,
    Delete,
    Render,
    FileLoad,
    Navigation,
    Scroll,
}

impl Operation {
    /// 操作の目標時間を取得
    pub fn target_time(&self) -> Duration {
        match self {
            Operation::CursorMove => Duration::from_millis(1),
            Operation::Insert => Duration::from_millis(2),
            Operation::Delete => Duration::from_millis(2),
            Operation::Render => Duration::from_millis(16), // 60fps
            Operation::FileLoad => Duration::from_millis(100),
            Operation::Navigation => Duration::from_millis(1),
            Operation::Scroll => Duration::from_millis(5),
        }
    }
}

/// パフォーマンス統計
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub count: u32,
    pub violations: u32, // 目標時間を超過した回数
}

impl PerformanceStats {
    pub fn new() -> Self {
        Self {
            min: Duration::MAX,
            max: Duration::ZERO,
            avg: Duration::ZERO,
            count: 0,
            violations: 0,
        }
    }

    pub fn update(&mut self, duration: Duration, target: Duration) {
        self.count += 1;

        if duration < self.min {
            self.min = duration;
        }
        if duration > self.max {
            self.max = duration;
        }

        // 移動平均の計算
        let total_nanos = self.avg.as_nanos() as u64 * (self.count - 1) as u64 + duration.as_nanos() as u64;
        self.avg = Duration::from_nanos(total_nanos / self.count as u64);

        if duration > target {
            self.violations += 1;
        }
    }

    /// 目標達成率を計算
    pub fn success_rate(&self) -> f32 {
        if self.count == 0 {
            return 1.0;
        }
        (self.count - self.violations) as f32 / self.count as f32
    }
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self::new()
    }
}

/// パフォーマンスメトリクス
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 操作別統計
    pub stats: HashMap<Operation, PerformanceStats>,
    /// メモリ使用量（バイト）
    pub memory_usage: usize,
    /// フレームレート
    pub frame_rate: f32,
    /// 最後の更新時刻
    pub last_update: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            memory_usage: 0,
            frame_rate: 0.0,
            last_update: Instant::now(),
        }
    }

    /// 統計を取得または作成
    pub fn get_or_create_stats(&mut self, operation: Operation) -> &mut PerformanceStats {
        self.stats.entry(operation).or_insert_with(PerformanceStats::new)
    }

    /// 全体的な健康度を計算
    pub fn health_score(&self) -> f32 {
        if self.stats.is_empty() {
            return 1.0;
        }

        let total_score: f32 = self.stats.values()
            .map(|stats| stats.success_rate())
            .sum();

        total_score / self.stats.len() as f32
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// パフォーマンス監視マネージャー
#[derive(Debug)]
pub struct PerformanceMonitor {
    metrics: PerformanceMetrics,
    enabled: bool,
    #[allow(dead_code)]
    max_samples: usize,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: PerformanceMetrics::new(),
            enabled: true,
            max_samples: 1000,
        }
    }

    /// 監視の有効/無効を切り替え
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 操作の計測を開始
    pub fn start_operation(&self, _operation: Operation) -> OperationTimer {
        if self.enabled {
            OperationTimer::new(_operation)
        } else {
            OperationTimer::disabled()
        }
    }

    /// 操作の計測結果を記録
    pub fn record_operation(&mut self, operation: Operation, duration: Duration) {
        if !self.enabled {
            return;
        }

        let target = operation.target_time();
        let stats = self.metrics.get_or_create_stats(operation);
        stats.update(duration, target);

        self.metrics.last_update = Instant::now();
    }

    /// メモリ使用量を更新
    pub fn update_memory_usage(&mut self, bytes: usize) {
        self.metrics.memory_usage = bytes;
    }

    /// フレームレートを更新
    pub fn update_frame_rate(&mut self, fps: f32) {
        self.metrics.frame_rate = fps;
    }

    /// 現在のメトリクスを取得
    pub fn metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// 警告が必要な操作を検出
    pub fn detect_warnings(&self) -> Vec<(Operation, String)> {
        let mut warnings = Vec::new();

        for (operation, stats) in &self.metrics.stats {
            if stats.success_rate() < 0.9 {
                warnings.push((*operation, format!(
                    "成功率が低下: {:.1}% (目標: 90%+)",
                    stats.success_rate() * 100.0
                )));
            }

            if stats.avg > operation.target_time() * 2 {
                warnings.push((*operation, format!(
                    "平均時間が目標の2倍を超過: {:.2}ms (目標: {:.2}ms)",
                    stats.avg.as_secs_f64() * 1000.0,
                    operation.target_time().as_secs_f64() * 1000.0
                )));
            }
        }

        // メモリ使用量チェック
        if self.metrics.memory_usage > 10 * 1024 * 1024 {
            warnings.push((Operation::FileLoad, format!(
                "メモリ使用量が目標を超過: {:.1}MB (目標: 10MB以下)",
                self.metrics.memory_usage as f64 / (1024.0 * 1024.0)
            )));
        }

        // フレームレートチェック
        if self.metrics.frame_rate < 55.0 && self.metrics.frame_rate > 0.0 {
            warnings.push((Operation::Render, format!(
                "フレームレートが低下: {:.1}fps (目標: 60fps)",
                self.metrics.frame_rate
            )));
        }

        warnings
    }

    /// レポートを生成
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== パフォーマンスレポート ===\n\n");

        // 全体健康度
        report.push_str(&format!("健康度: {:.1}%\n", self.metrics.health_score() * 100.0));
        report.push_str(&format!("メモリ使用量: {:.2}MB\n", self.metrics.memory_usage as f64 / (1024.0 * 1024.0)));
        report.push_str(&format!("フレームレート: {:.1}fps\n\n", self.metrics.frame_rate));

        // 操作別統計
        report.push_str("=== 操作別統計 ===\n");
        for (operation, stats) in &self.metrics.stats {
            let target_ms = operation.target_time().as_secs_f64() * 1000.0;
            let avg_ms = stats.avg.as_secs_f64() * 1000.0;
            let min_ms = stats.min.as_secs_f64() * 1000.0;
            let max_ms = stats.max.as_secs_f64() * 1000.0;

            report.push_str(&format!(
                "{:?}: 平均 {:.2}ms (目標 {:.2}ms), 範囲 {:.2}-{:.2}ms, 成功率 {:.1}%, サンプル数 {}\n",
                operation, avg_ms, target_ms, min_ms, max_ms,
                stats.success_rate() * 100.0, stats.count
            ));
        }

        // 警告
        let warnings = self.detect_warnings();
        if !warnings.is_empty() {
            report.push_str("\n=== 警告 ===\n");
            for (operation, message) in warnings {
                report.push_str(&format!("{:?}: {}\n", operation, message));
            }
        }

        report
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 操作タイマー
pub struct OperationTimer {
    operation: Option<Operation>,
    start_time: Instant,
}

impl OperationTimer {
    fn new(operation: Operation) -> Self {
        Self {
            operation: Some(operation),
            start_time: Instant::now(),
        }
    }

    fn disabled() -> Self {
        Self {
            operation: None,
            start_time: Instant::now(),
        }
    }

    /// タイマーを停止して結果を返す
    pub fn finish(self, monitor: &mut PerformanceMonitor) {
        if let Some(operation) = self.operation {
            let duration = self.start_time.elapsed();
            monitor.record_operation(operation, duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_stats() {
        let mut stats = PerformanceStats::new();
        let target = Duration::from_millis(1);

        // 目標以内の操作
        stats.update(Duration::from_micros(500), target);
        assert_eq!(stats.violations, 0);
        assert_eq!(stats.success_rate(), 1.0);

        // 目標超過の操作
        stats.update(Duration::from_millis(2), target);
        assert_eq!(stats.violations, 1);
        assert_eq!(stats.success_rate(), 0.5);
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        // 短時間の操作をシミュレート
        monitor.record_operation(Operation::CursorMove, Duration::from_micros(500));

        let metrics = monitor.metrics();
        let cursor_stats = metrics.stats.get(&Operation::CursorMove).unwrap();
        assert_eq!(cursor_stats.count, 1);
        assert_eq!(cursor_stats.violations, 0);
    }

    #[test]
    fn test_operation_timer() {
        let mut monitor = PerformanceMonitor::new();
        let timer = monitor.start_operation(Operation::CursorMove);

        thread::sleep(Duration::from_millis(1));
        timer.finish(&mut monitor);

        let metrics = monitor.metrics();
        assert!(metrics.stats.contains_key(&Operation::CursorMove));
    }

    #[test]
    fn test_health_score() {
        let mut monitor = PerformanceMonitor::new();

        // 良好な操作
        monitor.record_operation(Operation::CursorMove, Duration::from_micros(500));
        monitor.record_operation(Operation::Navigation, Duration::from_micros(800));

        let health = monitor.metrics().health_score();
        assert!(health > 0.9);

        // 悪い操作を追加
        monitor.record_operation(Operation::CursorMove, Duration::from_millis(10));

        let health_after = monitor.metrics().health_score();
        assert!(health_after < health);
    }
}