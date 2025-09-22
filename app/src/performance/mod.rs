//! パフォーマンス最適化モジュール
//!
//! パフォーマンス監視、最適化、プロファイリング機能を提供

pub mod monitor;
pub mod optimization;
pub mod profiling;

// 公開API
pub use monitor::{PerformanceMonitor, PerformanceMetrics, Operation};
pub use optimization::{OptimizationConfig, PerformanceOptimizer, LongLineStrategy};
pub use profiling::{ProfilerManager, ProfileResult};