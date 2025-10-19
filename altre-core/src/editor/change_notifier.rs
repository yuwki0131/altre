//! 変更通知システム
//!
//! エディタの変更イベントを効率的に配信・管理するシステム

use crate::buffer::{ChangeEvent, ChangeListener};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// リスナーの一意識別子
pub type ListenerId = usize;

/// 拡張変更イベント
#[derive(Debug, Clone)]
pub enum ExtendedChangeEvent {
    /// 基本変更イベント
    Base(ChangeEvent),

    /// バッチ操作開始
    BatchStart {
        operation_id: usize,
        expected_changes: usize,
    },

    /// バッチ操作終了
    BatchEnd {
        operation_id: usize,
        actual_changes: usize,
    },

    /// 大規模変更開始（リアルタイム更新を一時停止）
    BulkChangeStart,

    /// 大規模変更終了（リアルタイム更新を再開）
    BulkChangeEnd {
        total_changes: usize,
        duration: Duration,
    },

    /// パフォーマンス警告
    PerformanceWarning {
        operation: String,
        duration: Duration,
        threshold: Duration,
    },

    /// エラー発生
    Error {
        operation: String,
        error_msg: String,
    },

    /// 保存状態変更
    SaveStateChanged { is_modified: bool },

    /// 選択範囲変更
    SelectionChanged {
        old_selection: Option<(usize, usize)>,
        new_selection: Option<(usize, usize)>,
    },

    /// ビューポート変更
    ViewportChanged {
        old_viewport: ViewportInfo,
        new_viewport: ViewportInfo,
    },
}

/// ビューポート情報
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportInfo {
    /// 表示開始行
    pub start_line: usize,
    /// 表示終了行
    pub end_line: usize,
    /// 水平スクロール位置
    pub scroll_x: usize,
    /// 表示幅
    pub width: usize,
    /// 表示高さ
    pub height: usize,
}

impl ViewportInfo {
    pub fn new(
        start_line: usize,
        end_line: usize,
        scroll_x: usize,
        width: usize,
        height: usize,
    ) -> Self {
        Self {
            start_line,
            end_line,
            scroll_x,
            width,
            height,
        }
    }
}

/// 拡張変更リスナー
pub trait ExtendedChangeListener {
    /// 拡張変更イベントを処理
    fn on_extended_change(&mut self, event: &ExtendedChangeEvent);

    /// リスナーの優先度を返す（高い値ほど優先）
    fn priority(&self) -> i32 {
        0
    }

    /// 特定のイベント種別のみを処理するかどうか
    fn accepts_event_type(&self, event: &ExtendedChangeEvent) -> bool {
        let _ = event;
        true // デフォルトは全てのイベントを受け入れ
    }
}

/// 変更通知システムの統計情報
#[derive(Debug, Clone)]
pub struct ChangeNotifierStats {
    /// 総リスナー数
    pub total_listeners: usize,
    /// アクティブリスナー数
    pub active_listeners: usize,
    /// 配信イベント数
    pub events_dispatched: usize,
    /// 平均配信時間
    pub avg_dispatch_time: Duration,
    /// 最大配信時間
    pub max_dispatch_time: Duration,
    /// エラー数
    pub error_count: usize,
}

/// バッチ操作の情報
#[derive(Debug, Clone)]
pub struct BatchInfo {
    /// 操作ID
    pub operation_id: usize,
    /// 実行時間
    pub duration: Duration,
    /// 予想変更数
    pub expected_changes: usize,
    /// 実際の変更数
    pub actual_changes: usize,
    /// イベント数
    pub events_count: usize,
}

/// 高性能変更通知システム
pub struct AdvancedChangeNotifier {
    /// 基本リスナー（互換性のため）
    basic_listeners: Vec<Box<dyn ChangeListener>>,
    /// 拡張リスナー（優先度付き）
    extended_listeners: HashMap<ListenerId, (Box<dyn ExtendedChangeListener>, i32)>,
    /// リスナーIDカウンター
    next_listener_id: AtomicUsize,
    /// バッチ操作状態
    batch_state: Option<BatchState>,
    /// 大規模変更状態
    bulk_change_active: bool,
    /// パフォーマンス監視
    stats: ChangeNotifierStats,
    /// 最後の配信時刻
    last_dispatch_time: Instant,
    /// 配信タイムアウト（デバッグ用）
    dispatch_timeout: Duration,
    /// イベントフィルター
    event_filter: Option<Box<dyn Fn(&ExtendedChangeEvent) -> bool>>,
}

/// バッチ操作状態
#[derive(Debug)]
struct BatchState {
    operation_id: usize,
    start_time: Instant,
    expected_changes: usize,
    actual_changes: usize,
    batched_events: Vec<ExtendedChangeEvent>,
}

impl AdvancedChangeNotifier {
    /// 新しい変更通知システムを作成
    pub fn new() -> Self {
        Self {
            basic_listeners: Vec::new(),
            extended_listeners: HashMap::new(),
            next_listener_id: AtomicUsize::new(0),
            batch_state: None,
            bulk_change_active: false,
            stats: ChangeNotifierStats {
                total_listeners: 0,
                active_listeners: 0,
                events_dispatched: 0,
                avg_dispatch_time: Duration::ZERO,
                max_dispatch_time: Duration::ZERO,
                error_count: 0,
            },
            last_dispatch_time: Instant::now(),
            dispatch_timeout: Duration::from_millis(10), // 10ms timeout
            event_filter: None,
        }
    }

    /// 基本リスナーを追加（互換性）
    pub fn add_basic_listener(&mut self, listener: Box<dyn ChangeListener>) {
        self.basic_listeners.push(listener);
        self.update_listener_count();
    }

    /// 拡張リスナーを追加
    pub fn add_extended_listener(
        &mut self,
        listener: Box<dyn ExtendedChangeListener>,
    ) -> ListenerId {
        let id = self.next_listener_id.fetch_add(1, Ordering::SeqCst);
        let priority = listener.priority();
        self.extended_listeners.insert(id, (listener, priority));
        self.update_listener_count();
        id
    }

    /// リスナーを削除
    pub fn remove_listener(&mut self, id: ListenerId) -> bool {
        let removed = self.extended_listeners.remove(&id).is_some();
        if removed {
            self.update_listener_count();
        }
        removed
    }

    /// 基本変更イベントを通知
    pub fn notify_basic(&mut self, event: ChangeEvent) {
        let extended_event = ExtendedChangeEvent::Base(event.clone());

        // 基本リスナーに通知
        for listener in &mut self.basic_listeners {
            listener.on_change(&event);
        }

        // 拡張リスナーにも通知
        self.dispatch_extended_event(extended_event);
    }

    /// 拡張変更イベントを通知
    pub fn notify_extended(&mut self, event: ExtendedChangeEvent) {
        self.dispatch_extended_event(event);
    }

    /// バッチ操作を開始
    pub fn start_batch(&mut self, expected_changes: usize) -> usize {
        let operation_id = self.next_listener_id.fetch_add(1, Ordering::SeqCst);

        let event = ExtendedChangeEvent::BatchStart {
            operation_id,
            expected_changes,
        };

        self.batch_state = Some(BatchState {
            operation_id,
            start_time: Instant::now(),
            expected_changes,
            actual_changes: 0,
            batched_events: vec![event.clone()],
        });

        self.dispatch_extended_event(event);
        operation_id
    }

    /// バッチ操作を終了
    pub fn end_batch(&mut self, operation_id: usize) {
        if let Some(ref mut state) = self.batch_state {
            if state.operation_id == operation_id {
                let duration = state.start_time.elapsed();
                let actual_changes = state.actual_changes;
                let expected_changes = state.expected_changes;

                let event = ExtendedChangeEvent::BatchEnd {
                    operation_id,
                    actual_changes,
                };

                state.batched_events.push(event.clone());
                self.dispatch_extended_event(event);

                // パフォーマンス警告（バッチ操作が長時間の場合）
                if duration.as_millis() > 10 {
                    self.notify_performance_warning(
                        format!("batch_operation_{}", operation_id),
                        duration,
                        Duration::from_millis(10),
                    );
                }

                // 予想との差異が大きい場合の警告
                if expected_changes > 0 {
                    let difference_ratio = if actual_changes > expected_changes {
                        (actual_changes as f64 - expected_changes as f64) / expected_changes as f64
                    } else {
                        (expected_changes as f64 - actual_changes as f64) / expected_changes as f64
                    };

                    if difference_ratio > 0.5 {
                        eprintln!("Warning: Batch operation {} had significant difference between expected ({}) and actual ({}) changes",
                                 operation_id, expected_changes, actual_changes);
                    }
                }

                // バッチ状態をクリア
                self.batch_state = None;
            }
        }
    }

    /// 大規模変更を開始
    pub fn start_bulk_change(&mut self) {
        if !self.bulk_change_active {
            self.bulk_change_active = true;
            self.dispatch_extended_event(ExtendedChangeEvent::BulkChangeStart);
        }
    }

    /// 大規模変更を終了
    pub fn end_bulk_change(&mut self, total_changes: usize, duration: Duration) {
        if self.bulk_change_active {
            self.bulk_change_active = false;
            self.dispatch_extended_event(ExtendedChangeEvent::BulkChangeEnd {
                total_changes,
                duration,
            });
        }
    }

    /// イベントフィルターを設定
    pub fn set_event_filter<F>(&mut self, filter: F)
    where
        F: Fn(&ExtendedChangeEvent) -> bool + 'static,
    {
        self.event_filter = Some(Box::new(filter));
    }

    /// イベントフィルターをクリア
    pub fn clear_event_filter(&mut self) {
        self.event_filter = None;
    }

    /// 統計情報を取得
    pub fn stats(&self) -> &ChangeNotifierStats {
        &self.stats
    }

    /// 現在のバッチ操作の情報を取得
    pub fn current_batch_info(&self) -> Option<BatchInfo> {
        self.batch_state.as_ref().map(|state| BatchInfo {
            operation_id: state.operation_id,
            duration: state.start_time.elapsed(),
            expected_changes: state.expected_changes,
            actual_changes: state.actual_changes,
            events_count: state.batched_events.len(),
        })
    }

    /// 統計情報をリセット
    pub fn reset_stats(&mut self) {
        self.stats = ChangeNotifierStats {
            total_listeners: self.stats.total_listeners,
            active_listeners: self.stats.active_listeners,
            events_dispatched: 0,
            avg_dispatch_time: Duration::ZERO,
            max_dispatch_time: Duration::ZERO,
            error_count: 0,
        };
    }

    /// パフォーマンス警告を通知
    pub fn notify_performance_warning(
        &mut self,
        operation: String,
        duration: Duration,
        threshold: Duration,
    ) {
        let event = ExtendedChangeEvent::PerformanceWarning {
            operation,
            duration,
            threshold,
        };
        self.dispatch_extended_event(event);
    }

    /// エラーを通知
    pub fn notify_error(&mut self, operation: String, error_msg: String) {
        let event = ExtendedChangeEvent::Error {
            operation,
            error_msg,
        };
        self.dispatch_extended_event(event);
    }

    /// 内部配信処理
    fn dispatch_extended_event(&mut self, event: ExtendedChangeEvent) {
        let start_time = Instant::now();

        // フィルターチェック
        if let Some(ref filter) = self.event_filter {
            if !filter(&event) {
                return; // フィルターで除外
            }
        }

        // バッチ状態の更新
        if let Some(ref mut state) = self.batch_state {
            match &event {
                ExtendedChangeEvent::Base(_) => {
                    state.actual_changes += 1;
                    state.batched_events.push(event.clone());
                }
                _ => {}
            }
        }

        // 優先度順にソートされたリスナーリストを取得
        let mut sorted_listeners: Vec<_> = self.extended_listeners.iter_mut().collect();
        sorted_listeners.sort_by(|a, b| b.1 .1.cmp(&a.1 .1)); // 優先度の降順

        // リスナーに配信
        for (_, (listener, _)) in sorted_listeners {
            if listener.accepts_event_type(&event) {
                listener.on_extended_change(&event);
            }
        }

        // 統計更新
        let dispatch_time = start_time.elapsed();
        self.update_dispatch_stats(dispatch_time);

        // タイムアウト警告
        if dispatch_time > self.dispatch_timeout {
            eprintln!(
                "Warning: Event dispatch took {}ms (timeout: {}ms)",
                dispatch_time.as_millis(),
                self.dispatch_timeout.as_millis()
            );
        }

        self.last_dispatch_time = Instant::now();
    }

    /// リスナー数を更新
    fn update_listener_count(&mut self) {
        self.stats.total_listeners = self.basic_listeners.len() + self.extended_listeners.len();
        self.stats.active_listeners = self.stats.total_listeners; // 簡略化
    }

    /// 配信統計を更新
    fn update_dispatch_stats(&mut self, duration: Duration) {
        self.stats.events_dispatched += 1;

        // 平均時間の更新（移動平均）
        let weight = 0.1; // 10%の重み
        if self.stats.avg_dispatch_time.as_nanos() == 0 {
            self.stats.avg_dispatch_time = duration;
        } else {
            let current_nanos = self.stats.avg_dispatch_time.as_nanos() as f64;
            let new_nanos = duration.as_nanos() as f64;
            let avg_nanos = current_nanos * (1.0 - weight) + new_nanos * weight;
            self.stats.avg_dispatch_time = Duration::from_nanos(avg_nanos as u64);
        }

        // 最大時間の更新
        if duration > self.stats.max_dispatch_time {
            self.stats.max_dispatch_time = duration;
        }
    }
}

impl Default for AdvancedChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}

/// テスト用リスナー
#[cfg(test)]
pub struct TestListener {
    pub events: Vec<ExtendedChangeEvent>,
    pub priority: i32,
}

#[cfg(test)]
impl TestListener {
    pub fn new(priority: i32) -> Self {
        Self {
            events: Vec::new(),
            priority,
        }
    }
}

#[cfg(test)]
impl ExtendedChangeListener for TestListener {
    fn on_extended_change(&mut self, event: &ExtendedChangeEvent) {
        self.events.push(event.clone());
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::CursorPosition;

    #[test]
    fn test_advanced_change_notifier_creation() {
        let notifier = AdvancedChangeNotifier::new();
        assert_eq!(notifier.stats().total_listeners, 0);
        assert!(!notifier.bulk_change_active);
    }

    #[test]
    fn test_add_extended_listener() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(10);

        let id = notifier.add_extended_listener(Box::new(listener));
        assert_eq!(notifier.stats().total_listeners, 1);

        // リスナー削除
        assert!(notifier.remove_listener(id));
        assert_eq!(notifier.stats().total_listeners, 0);
    }

    #[test]
    fn test_basic_event_notification() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(0);
        let _id = notifier.add_extended_listener(Box::new(listener));

        let _cursor = CursorPosition::new();
        let event = ChangeEvent::Insert {
            position: 0,
            content: "test".to_string(),
        };

        notifier.notify_basic(event);
        assert_eq!(notifier.stats().events_dispatched, 1);
    }

    #[test]
    fn test_batch_operations() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(0);
        let _id = notifier.add_extended_listener(Box::new(listener));

        let batch_id = notifier.start_batch(3);
        assert!(notifier.batch_state.is_some());

        // バッチ情報を確認
        let batch_info = notifier.current_batch_info().unwrap();
        assert_eq!(batch_info.operation_id, batch_id);
        assert_eq!(batch_info.expected_changes, 3);
        assert_eq!(batch_info.actual_changes, 0);

        // バッチ内でイベント発生
        let event = ChangeEvent::Insert {
            position: 0,
            content: "a".to_string(),
        };
        notifier.notify_basic(event);

        // バッチ情報を再確認
        let batch_info = notifier.current_batch_info().unwrap();
        assert_eq!(batch_info.actual_changes, 1);

        notifier.end_batch(batch_id);
        assert!(notifier.batch_state.is_none());
        assert!(notifier.current_batch_info().is_none());
    }

    #[test]
    fn test_bulk_change_operations() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(0);
        let _id = notifier.add_extended_listener(Box::new(listener));

        notifier.start_bulk_change();
        assert!(notifier.bulk_change_active);

        notifier.end_bulk_change(100, Duration::from_millis(50));
        assert!(!notifier.bulk_change_active);
        assert!(notifier.stats().events_dispatched >= 2); // start + end
    }

    #[test]
    fn test_listener_priority() {
        let mut notifier = AdvancedChangeNotifier::new();

        // 異なる優先度のリスナーを追加
        let high_priority = TestListener::new(100);
        let low_priority = TestListener::new(1);

        let _high_id = notifier.add_extended_listener(Box::new(high_priority));
        let _low_id = notifier.add_extended_listener(Box::new(low_priority));

        // イベントを配信
        let event = ExtendedChangeEvent::BulkChangeStart;
        notifier.notify_extended(event);

        // 優先度の高いリスナーが先に処理されることを確認
        // （実際のテストでは内部状態の検証が必要）
        assert_eq!(notifier.stats().total_listeners, 2);
    }

    #[test]
    fn test_performance_warning() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(0);
        let _id = notifier.add_extended_listener(Box::new(listener));

        notifier.notify_performance_warning(
            "test_operation".to_string(),
            Duration::from_millis(5),
            Duration::from_millis(1),
        );

        assert!(notifier.stats().events_dispatched > 0);
    }

    #[test]
    fn test_event_filter() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(0);
        let _id = notifier.add_extended_listener(Box::new(listener));

        // エラーイベントのみを通すフィルターを設定
        notifier.set_event_filter(|event| matches!(event, ExtendedChangeEvent::Error { .. }));

        // 通常イベント（フィルターされる）
        notifier.notify_extended(ExtendedChangeEvent::BulkChangeStart);

        // エラーイベント（通る）
        notifier.notify_error("test".to_string(), "error".to_string());

        // エラーイベントのみがカウントされる
        assert_eq!(notifier.stats().events_dispatched, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut notifier = AdvancedChangeNotifier::new();
        let listener = TestListener::new(0);
        let _id = notifier.add_extended_listener(Box::new(listener));

        // いくつかのイベントを配信
        for i in 0..5 {
            notifier.notify_error(format!("op_{}", i), "test error".to_string());
        }

        let stats = notifier.stats();
        assert_eq!(stats.events_dispatched, 5);
        assert!(stats.avg_dispatch_time.as_nanos() > 0);

        // 統計リセット
        notifier.reset_stats();
        assert_eq!(notifier.stats().events_dispatched, 0);
    }
}
