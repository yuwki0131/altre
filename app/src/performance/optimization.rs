//! パフォーマンス最適化機能
//!
//! 動的な最適化設定とパフォーマンス調整機能を提供

use std::collections::HashMap;

/// 最適化設定
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// 長い行の閾値（文字数）
    pub long_line_threshold: usize,
    /// 超長い行の閾値（文字数）
    pub very_long_line_threshold: usize,
    /// ギャップバッファの初期サイズ
    pub gap_buffer_initial_size: usize,
    /// ギャップバッファの最大サイズ
    pub gap_buffer_max_size: usize,
    /// キャッシュサイズの上限
    pub cache_size_limit: usize,
    /// レンダリング最適化レベル
    pub render_optimization_level: RenderOptimizationLevel,
    /// メモリプールサイズ
    pub memory_pool_size: usize,
}

impl OptimizationConfig {
    pub fn new() -> Self {
        Self {
            long_line_threshold: 1000,
            very_long_line_threshold: 10000,
            gap_buffer_initial_size: 4096,
            gap_buffer_max_size: 1024 * 1024,
            cache_size_limit: 100,
            render_optimization_level: RenderOptimizationLevel::Balanced,
            memory_pool_size: 1024 * 1024,
        }
    }

    /// 保守的な設定（安全性重視）
    pub fn conservative() -> Self {
        Self {
            long_line_threshold: 500,
            very_long_line_threshold: 5000,
            gap_buffer_initial_size: 2048,
            gap_buffer_max_size: 512 * 1024,
            cache_size_limit: 50,
            render_optimization_level: RenderOptimizationLevel::Safe,
            memory_pool_size: 512 * 1024,
        }
    }

    /// 高性能設定（速度重視）
    pub fn high_performance() -> Self {
        Self {
            long_line_threshold: 2000,
            very_long_line_threshold: 20000,
            gap_buffer_initial_size: 8192,
            gap_buffer_max_size: 2048 * 1024,
            cache_size_limit: 200,
            render_optimization_level: RenderOptimizationLevel::Aggressive,
            memory_pool_size: 2048 * 1024,
        }
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// レンダリング最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOptimizationLevel {
    /// 安全性重視（最小限の最適化）
    Safe,
    /// バランス重視（標準的な最適化）
    Balanced,
    /// 性能重視（積極的な最適化）
    Aggressive,
}

/// 長い行の最適化戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongLineStrategy {
    /// 通常処理
    Normal,
    /// チャンク処理
    Chunked,
    /// 段階的制限
    GradualLimitation,
    /// 表示制限
    DisplayLimited,
}

/// 最適化統計
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    /// 長い行の検出回数
    pub long_lines_detected: u32,
    /// 最適化モード切替回数
    pub optimization_switches: u32,
    /// メモリ節約量（バイト）
    pub memory_saved: usize,
    /// 時間節約量（マイクロ秒）
    pub time_saved_micros: u64,
}

impl OptimizationStats {
    pub fn new() -> Self {
        Self {
            long_lines_detected: 0,
            optimization_switches: 0,
            memory_saved: 0,
            time_saved_micros: 0,
        }
    }
}

impl Default for OptimizationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// パフォーマンス最適化マネージャー
#[derive(Debug)]
pub struct PerformanceOptimizer {
    config: OptimizationConfig,
    stats: OptimizationStats,
    current_strategy: HashMap<usize, LongLineStrategy>, // 行番号 -> 戦略
    enabled: bool,
}

impl PerformanceOptimizer {
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            config,
            stats: OptimizationStats::new(),
            current_strategy: HashMap::new(),
            enabled: true,
        }
    }

    /// 最適化の有効/無効を切り替え
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 設定を取得
    pub fn config(&self) -> &OptimizationConfig {
        &self.config
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: OptimizationConfig) {
        self.config = config;
        // 設定変更時は戦略をリセット
        self.current_strategy.clear();
    }

    /// 統計を取得
    pub fn stats(&self) -> &OptimizationStats {
        &self.stats
    }

    /// 長い行に対する最適化戦略を決定
    pub fn determine_long_line_strategy(&mut self, line_length: usize, line_number: usize) -> LongLineStrategy {
        if !self.enabled {
            return LongLineStrategy::Normal;
        }

        let strategy = if line_length < self.config.long_line_threshold {
            LongLineStrategy::Normal
        } else if line_length < self.config.very_long_line_threshold {
            LongLineStrategy::Chunked
        } else {
            match self.config.render_optimization_level {
                RenderOptimizationLevel::Safe => LongLineStrategy::GradualLimitation,
                RenderOptimizationLevel::Balanced => LongLineStrategy::Chunked,
                RenderOptimizationLevel::Aggressive => LongLineStrategy::DisplayLimited,
            }
        };

        // 戦略が変更された場合は統計を更新
        if let Some(&old_strategy) = self.current_strategy.get(&line_number) {
            if old_strategy != strategy {
                self.stats.optimization_switches += 1;
            }
        } else if strategy != LongLineStrategy::Normal {
            self.stats.long_lines_detected += 1;
        }

        self.current_strategy.insert(line_number, strategy);
        strategy
    }

    /// ギャップバッファサイズを最適化
    pub fn optimize_gap_buffer_size(&self, current_content_size: usize, recent_growth: usize) -> usize {
        if !self.enabled {
            return self.config.gap_buffer_initial_size;
        }

        // 内容量の1.5倍 + 最近の成長量の2倍を基準とする
        let base_size = (current_content_size * 3) / 2 + recent_growth * 2;

        // 最小値と最大値で制限
        base_size
            .max(self.config.gap_buffer_initial_size)
            .min(self.config.gap_buffer_max_size)
    }

    /// メモリ使用量を最適化
    pub fn optimize_memory_usage(&mut self, current_usage: usize) -> Vec<MemoryOptimization> {
        if !self.enabled {
            return Vec::new();
        }

        let mut optimizations = Vec::new();

        // メモリ使用量が閾値を超えた場合の対策
        let threshold = 8 * 1024 * 1024; // 8MB
        if current_usage > threshold {
            optimizations.push(MemoryOptimization::CompactGapBuffer);
            optimizations.push(MemoryOptimization::ClearCaches);

            if current_usage > threshold * 2 {
                optimizations.push(MemoryOptimization::ReduceBufferSize);
            }
        }

        // キャッシュサイズを調整
        if self.current_strategy.len() > self.config.cache_size_limit {
            optimizations.push(MemoryOptimization::TrimCaches);
        }

        optimizations
    }

    /// 描画最適化の推奨設定を取得
    pub fn get_render_optimizations(&self, _viewport_size: (usize, usize), content_size: usize) -> Vec<RenderOptimization> {
        if !self.enabled {
            return Vec::new();
        }

        let mut optimizations = Vec::new();

        match self.config.render_optimization_level {
            RenderOptimizationLevel::Safe => {
                optimizations.push(RenderOptimization::ViewportCulling);
            }
            RenderOptimizationLevel::Balanced => {
                optimizations.push(RenderOptimization::ViewportCulling);
                optimizations.push(RenderOptimization::DifferentialUpdate);
            }
            RenderOptimizationLevel::Aggressive => {
                optimizations.push(RenderOptimization::ViewportCulling);
                optimizations.push(RenderOptimization::DifferentialUpdate);
                optimizations.push(RenderOptimization::FrameBuffering);

                // 大きなファイルの場合はさらに最適化
                if content_size > 100_000 {
                    optimizations.push(RenderOptimization::LazyRendering);
                }
            }
        }

        optimizations
    }

    /// パフォーマンス統計をリセット
    pub fn reset_stats(&mut self) {
        self.stats = OptimizationStats::new();
    }

    /// 時間節約を記録
    pub fn record_time_saved(&mut self, microseconds: u64) {
        self.stats.time_saved_micros += microseconds;
    }

    /// メモリ節約を記録
    pub fn record_memory_saved(&mut self, bytes: usize) {
        self.stats.memory_saved += bytes;
    }
}

impl Default for PerformanceOptimizer {
    fn default() -> Self {
        Self::new(OptimizationConfig::default())
    }
}

/// メモリ最適化の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOptimization {
    /// ギャップバッファの圧縮
    CompactGapBuffer,
    /// キャッシュのクリア
    ClearCaches,
    /// バッファサイズの削減
    ReduceBufferSize,
    /// キャッシュのトリミング
    TrimCaches,
}

/// 描画最適化の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOptimization {
    /// ビューポートカリング
    ViewportCulling,
    /// 差分更新
    DifferentialUpdate,
    /// フレームバッファリング
    FrameBuffering,
    /// 遅延レンダリング
    LazyRendering,
}

/// 長い行のチャンク処理ユーティリティ
pub struct LongLineProcessor {
    chunk_size: usize,
}

impl LongLineProcessor {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// 長い行をチャンクに分割
    pub fn split_into_chunks<'a>(&self, text: &'a str) -> Vec<&'a str> {
        if text.len() <= self.chunk_size {
            return vec![text];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = (start + self.chunk_size).min(text.len());

            // UTF-8境界で安全に分割
            let chunk_end = self.find_safe_split_point(text, start, end);
            chunks.push(&text[start..chunk_end]);
            start = chunk_end;
        }

        chunks
    }

    /// UTF-8境界で安全な分割点を見つける
    fn find_safe_split_point(&self, text: &str, start: usize, proposed_end: usize) -> usize {
        if proposed_end >= text.len() {
            return text.len();
        }

        // UTF-8境界を探す
        for i in (start..=proposed_end).rev() {
            if text.is_char_boundary(i) {
                return i;
            }
        }

        // 見つからない場合は開始点を返す（安全のため）
        start
    }

    /// チャンクサイズを調整
    pub fn adjust_chunk_size(&mut self, new_size: usize) {
        self.chunk_size = new_size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_config() {
        let config = OptimizationConfig::new();
        assert_eq!(config.long_line_threshold, 1000);

        let conservative = OptimizationConfig::conservative();
        assert!(conservative.long_line_threshold < config.long_line_threshold);

        let high_perf = OptimizationConfig::high_performance();
        assert!(high_perf.long_line_threshold > config.long_line_threshold);
    }

    #[test]
    fn test_long_line_strategy() {
        let mut optimizer = PerformanceOptimizer::new(OptimizationConfig::new());

        // 短い行
        let strategy = optimizer.determine_long_line_strategy(500, 0);
        assert_eq!(strategy, LongLineStrategy::Normal);

        // 長い行
        let strategy = optimizer.determine_long_line_strategy(1500, 1);
        assert_eq!(strategy, LongLineStrategy::Chunked);

        // 超長い行
        let strategy = optimizer.determine_long_line_strategy(15000, 2);
        assert_ne!(strategy, LongLineStrategy::Normal);
    }

    #[test]
    fn test_gap_buffer_optimization() {
        let optimizer = PerformanceOptimizer::new(OptimizationConfig::new());

        let size = optimizer.optimize_gap_buffer_size(1000, 100);
        assert!(size >= optimizer.config.gap_buffer_initial_size);
        assert!(size <= optimizer.config.gap_buffer_max_size);
    }

    #[test]
    fn test_long_line_processor() {
        let processor = LongLineProcessor::new(10);
        let text = "あいうえおかきくけこさしすせそ"; // 15文字

        let chunks = processor.split_into_chunks(text);
        assert!(chunks.len() > 1);

        // すべてのチャンクが結合されると元のテキストになる
        let rejoined = chunks.join("");
        assert_eq!(rejoined, text);
    }

    #[test]
    fn test_memory_optimization() {
        let mut optimizer = PerformanceOptimizer::new(OptimizationConfig::new());

        // 高メモリ使用量
        let optimizations = optimizer.optimize_memory_usage(10 * 1024 * 1024);
        assert!(!optimizations.is_empty());
        assert!(optimizations.contains(&MemoryOptimization::CompactGapBuffer));
    }

    #[test]
    fn test_render_optimizations() {
        let optimizer = PerformanceOptimizer::new(
            OptimizationConfig {
                render_optimization_level: RenderOptimizationLevel::Aggressive,
                ..OptimizationConfig::new()
            }
        );

        let optimizations = optimizer.get_render_optimizations((80, 24), 200_000);
        assert!(optimizations.contains(&RenderOptimization::ViewportCulling));
        assert!(optimizations.contains(&RenderOptimization::LazyRendering));
    }
}