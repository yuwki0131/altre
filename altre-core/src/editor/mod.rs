//! エディタモジュール
//!
//! 基本編集機能の統合モジュール

pub mod change_notifier;
pub mod edit_operations;
pub mod history;
pub mod input_buffer;
pub mod kill_ring;
pub mod text_editor;

// 公開API
pub use change_notifier::{
    AdvancedChangeNotifier, BatchInfo, ChangeNotifierStats, ExtendedChangeEvent,
    ExtendedChangeListener, ListenerId, ViewportInfo,
};
pub use edit_operations::{
    utils as edit_utils, EditContext, EditMetrics, EditMode, ExtendedEditOperations,
    OperationResult,
};
pub use history::{AtomicEdit, HistoryCommandKind, HistoryEntry, HistoryManager, HistoryStack};
pub use input_buffer::{InputBuffer, InputBufferError, InputBufferStats};
pub use kill_ring::KillRing;
pub use text_editor::TextEditor;

// 互換性のため、bufferモジュールから必要な型を再エクスポート
pub use crate::buffer::{
    ChangeEvent, ChangeListener, CursorPosition, EditOperations, NavigationAction, NavigationError,
    NavigationPosition,
};
pub use crate::error::{EditError, Result};
