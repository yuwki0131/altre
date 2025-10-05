//! プロファイリングシステム
//!
//! 詳細なパフォーマンス分析とプロファイリング機能を提供

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

/// プロファイル結果
#[derive(Debug, Clone)]
pub struct ProfileResult {
    /// 関数名
    pub function_name: String,
    /// 実行時間
    pub duration: Duration,
    /// 呼び出し回数
    pub call_count: u32,
    /// 子関数の結果
    pub children: Vec<ProfileResult>,
    /// 開始時刻
    pub start_time: Instant,
}

impl ProfileResult {
    pub fn new(function_name: String, start_time: Instant) -> Self {
        Self {
            function_name,
            duration: Duration::ZERO,
            call_count: 1,
            children: Vec::new(),
            start_time,
        }
    }

    /// 実行を完了し、時間を記録
    pub fn finish(&mut self) {
        self.duration = self.start_time.elapsed();
    }

    /// 子関数の結果を追加
    pub fn add_child(&mut self, child: ProfileResult) {
        self.children.push(child);
    }

    /// 総実行時間を計算（子関数を含む）
    pub fn total_duration(&self) -> Duration {
        self.duration + self.children.iter()
            .map(|child| child.total_duration())
            .sum::<Duration>()
    }

    /// 自身の実行時間を計算（子関数を除く）
    pub fn self_duration(&self) -> Duration {
        let children_duration: Duration = self.children.iter()
            .map(|child| child.duration)
            .sum();
        self.duration.saturating_sub(children_duration)
    }
}

impl fmt::Display for ProfileResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_with_indent(f, 0)
    }
}

impl ProfileResult {
    fn format_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let indent_str = "  ".repeat(indent);
        writeln!(f, "{}{}({}): {:.3}ms (self: {:.3}ms) [{}回]",
                 indent_str,
                 self.function_name,
                 self.call_count,
                 self.total_duration().as_secs_f64() * 1000.0,
                 self.self_duration().as_secs_f64() * 1000.0,
                 self.call_count)?;

        for child in &self.children {
            child.format_with_indent(f, indent + 1)?;
        }

        Ok(())
    }
}

/// プロファイラー状態
#[derive(Debug)]
struct ProfilerState {
    /// 現在実行中のプロファイル
    call_stack: Vec<ProfileResult>,
    /// 完了したプロファイル結果
    completed_profiles: HashMap<String, Vec<ProfileResult>>,
    /// 有効フラグ
    enabled: bool,
}

impl ProfilerState {
    fn new() -> Self {
        Self {
            call_stack: Vec::new(),
            completed_profiles: HashMap::new(),
            enabled: false,
        }
    }
}

/// プロファイラーマネージャー
#[derive(Debug)]
pub struct ProfilerManager {
    state: ProfilerState,
}

impl ProfilerManager {
    pub fn new() -> Self {
        Self {
            state: ProfilerState::new(),
        }
    }

    /// プロファイリングを開始
    pub fn start_profiling(&mut self) {
        self.state.enabled = true;
    }

    /// プロファイリングを停止
    pub fn stop_profiling(&mut self) {
        self.state.enabled = false;
        // 残っているスタックをクリア
        self.state.call_stack.clear();
    }

    /// プロファイリングが有効かチェック
    pub fn is_profiling(&self) -> bool {
        self.state.enabled
    }

    /// 関数の実行を開始
    pub fn enter_function(&mut self, function_name: &str) -> ProfileScope {
        if !self.state.enabled {
            return ProfileScope::disabled();
        }

        let profile = ProfileResult::new(function_name.to_string(), Instant::now());
        self.state.call_stack.push(profile);

        ProfileScope::new(function_name.to_string())
    }

    /// 関数の実行を終了
    pub fn exit_function(&mut self, function_name: &str) {
        if !self.state.enabled || self.state.call_stack.is_empty() {
            return;
        }

        if let Some(mut profile) = self.state.call_stack.pop() {
            profile.finish();

            // 親関数に追加するか、完了リストに追加
            if let Some(parent) = self.state.call_stack.last_mut() {
                parent.add_child(profile);
            } else {
                // ルートレベルの関数
                self.state.completed_profiles
                    .entry(function_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(profile);
            }
        }
    }

    /// 結果を取得
    pub fn get_results(&self) -> &HashMap<String, Vec<ProfileResult>> {
        &self.state.completed_profiles
    }

    /// 結果をクリア
    pub fn clear_results(&mut self) {
        self.state.completed_profiles.clear();
    }

    /// サマリーレポートを生成
    pub fn generate_summary(&self) -> String {
        let mut report = String::new();
        report.push_str("=== プロファイリングサマリー ===\n\n");

        // 関数別の統計を計算
        let mut function_stats: HashMap<String, (Duration, u32)> = HashMap::new();

        for (function_name, results) in &self.state.completed_profiles {
            let total_duration: Duration = results.iter()
                .map(|r| r.total_duration())
                .sum();
            let total_calls: u32 = results.iter()
                .map(|r| r.call_count)
                .sum();

            function_stats.insert(function_name.clone(), (total_duration, total_calls));
        }

        // 実行時間順にソート
        let mut sorted_functions: Vec<_> = function_stats.iter().collect();
        sorted_functions.sort_by(|a, b| b.1.0.cmp(&a.1.0));

        report.push_str("関数別実行時間（降順）:\n");
        for (function_name, (duration, calls)) in sorted_functions {
            let avg_duration = duration.as_nanos() as f64 / *calls as f64;
            report.push_str(&format!(
                "  {}: {:.3}ms total, {:.3}ms avg, {} calls\n",
                function_name,
                duration.as_secs_f64() * 1000.0,
                avg_duration / 1_000_000.0,
                calls
            ));
        }

        report.push_str("\n詳細結果:\n");
        for (function_name, results) in &self.state.completed_profiles {
            report.push_str(&format!("\n=== {} ===\n", function_name));
            for result in results {
                report.push_str(&format!("{}\n", result));
            }
        }

        report
    }

    /// ホットスポットを検出
    pub fn detect_hotspots(&self, threshold_ms: f64) -> Vec<(String, Duration)> {
        let mut hotspots = Vec::new();

        for (function_name, results) in &self.state.completed_profiles {
            let total_duration: Duration = results.iter()
                .map(|r| r.total_duration())
                .sum();

            if total_duration.as_secs_f64() * 1000.0 > threshold_ms {
                hotspots.push((function_name.clone(), total_duration));
            }
        }

        // 実行時間順にソート
        hotspots.sort_by(|a, b| b.1.cmp(&a.1));
        hotspots
    }
}

impl Default for ProfilerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// プロファイルスコープ（RAII）
pub struct ProfileScope {
    #[allow(dead_code)]
    function_name: Option<String>,
}

impl ProfileScope {
    fn new(function_name: String) -> Self {
        Self {
            function_name: Some(function_name),
        }
    }

    fn disabled() -> Self {
        Self {
            function_name: None,
        }
    }
}

impl Drop for ProfileScope {
    fn drop(&mut self) {
        // ProfileScopeが削除されるときは、プロファイラーは実際にはManagerで管理される
        // ここでは何もしない（Managerが直接exit_functionを呼ぶ）
    }
}

/// プロファイリングマクロ用のヘルパー
pub struct ScopedProfiler<'a> {
    manager: &'a mut ProfilerManager,
    function_name: String,
}

impl<'a> ScopedProfiler<'a> {
    pub fn new(manager: &'a mut ProfilerManager, function_name: &str) -> Self {
        manager.enter_function(function_name);
        Self {
            manager,
            function_name: function_name.to_string(),
        }
    }
}

impl<'a> Drop for ScopedProfiler<'a> {
    fn drop(&mut self) {
        self.manager.exit_function(&self.function_name);
    }
}

/// プロファイリングマクロ
#[macro_export]
macro_rules! profile_function {
    ($profiler:expr, $name:expr) => {
        let _profile_guard = $crate::performance::profiling::ScopedProfiler::new($profiler, $name);
    };
}

/// 複数のプロファイル結果を統合するユーティリティ
pub struct ProfileAggregator {
    aggregated_results: HashMap<String, AggregatedProfile>,
}

impl ProfileAggregator {
    pub fn new() -> Self {
        Self {
            aggregated_results: HashMap::new(),
        }
    }

    /// プロファイル結果を追加
    pub fn add_profile(&mut self, result: &ProfileResult) {
        let entry = self.aggregated_results
            .entry(result.function_name.clone())
            .or_insert_with(|| AggregatedProfile::new(result.function_name.clone()));

        entry.add_sample(result.total_duration(), result.call_count);
    }

    /// 統合結果を取得
    pub fn get_aggregated_results(&self) -> &HashMap<String, AggregatedProfile> {
        &self.aggregated_results
    }

    /// レポート生成
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== 統合プロファイリングレポート ===\n\n");

        let mut sorted_profiles: Vec<_> = self.aggregated_results.iter().collect();
        sorted_profiles.sort_by(|a, b| b.1.total_duration.cmp(&a.1.total_duration));

        for (_, profile) in sorted_profiles {
            report.push_str(&format!("{}\n", profile));
        }

        report
    }
}

impl Default for ProfileAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// 統合されたプロファイル結果
#[derive(Debug, Clone)]
pub struct AggregatedProfile {
    pub function_name: String,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
    pub sample_count: u32,
    pub total_calls: u32,
}

impl AggregatedProfile {
    fn new(function_name: String) -> Self {
        Self {
            function_name,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            sample_count: 0,
            total_calls: 0,
        }
    }

    fn add_sample(&mut self, duration: Duration, calls: u32) {
        self.sample_count += 1;
        self.total_calls += calls;
        self.total_duration += duration;

        if duration < self.min_duration {
            self.min_duration = duration;
        }
        if duration > self.max_duration {
            self.max_duration = duration;
        }

        self.avg_duration = Duration::from_nanos(
            self.total_duration.as_nanos() as u64 / self.sample_count as u64
        );
    }
}

impl fmt::Display for AggregatedProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: total {:.3}ms, avg {:.3}ms, min {:.3}ms, max {:.3}ms ({} samples, {} total calls)",
               self.function_name,
               self.total_duration.as_secs_f64() * 1000.0,
               self.avg_duration.as_secs_f64() * 1000.0,
               self.min_duration.as_secs_f64() * 1000.0,
               self.max_duration.as_secs_f64() * 1000.0,
               self.sample_count,
               self.total_calls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = ProfilerManager::new();
        assert!(!profiler.is_profiling());

        profiler.start_profiling();
        assert!(profiler.is_profiling());

        let _scope = profiler.enter_function("test_function");
        thread::sleep(Duration::from_millis(10));
        profiler.exit_function("test_function");

        let results = profiler.get_results();
        assert!(results.contains_key("test_function"));
    }

    #[test]
    fn test_nested_profiling() {
        let mut profiler = ProfilerManager::new();
        profiler.start_profiling();

        let _scope1 = profiler.enter_function("outer");
        let _scope2 = profiler.enter_function("inner");
        thread::sleep(Duration::from_millis(5));
        profiler.exit_function("inner");
        profiler.exit_function("outer");

        let results = profiler.get_results();
        assert!(results.contains_key("outer"));

        if let Some(outer_results) = results.get("outer") {
            assert!(!outer_results.is_empty());
            assert!(!outer_results[0].children.is_empty());
        }
    }

    #[test]
    fn test_hotspot_detection() {
        let mut profiler = ProfilerManager::new();
        profiler.start_profiling();

        // 長時間の関数をシミュレート
        let _scope = profiler.enter_function("slow_function");
        thread::sleep(Duration::from_millis(20));
        profiler.exit_function("slow_function");

        let hotspots = profiler.detect_hotspots(10.0);
        assert!(!hotspots.is_empty());
        assert_eq!(hotspots[0].0, "slow_function");
    }

    #[test]
    fn test_profile_aggregator() {
        let mut aggregator = ProfileAggregator::new();

        // サンプルプロファイル結果を作成
        let mut result1 = ProfileResult::new("test_func".to_string(), Instant::now());
        result1.duration = Duration::from_millis(10);
        result1.finish();

        let mut result2 = ProfileResult::new("test_func".to_string(), Instant::now());
        result2.duration = Duration::from_millis(20);
        result2.finish();

        aggregator.add_profile(&result1);
        aggregator.add_profile(&result2);

        let aggregated = aggregator.get_aggregated_results();
        assert!(aggregated.contains_key("test_func"));

        let test_func_profile = &aggregated["test_func"];
        assert_eq!(test_func_profile.sample_count, 2);
    }
}