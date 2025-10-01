//! エディタモジュール
//!
//! 基本編集機能の統合モジュール

pub mod input_buffer;
pub mod text_editor;
pub mod edit_operations;
pub mod change_notifier;
pub mod kill_ring;

// 公開API
pub use input_buffer::{InputBuffer, InputBufferError, InputBufferStats};
pub use text_editor::TextEditor;
pub use edit_operations::{
    ExtendedEditOperations, EditContext, EditMode, EditMetrics, OperationResult,
    utils as edit_utils,
};
pub use change_notifier::{
    AdvancedChangeNotifier, ExtendedChangeEvent, ExtendedChangeListener,
    ViewportInfo, ChangeNotifierStats, BatchInfo, ListenerId,
};
pub use kill_ring::KillRing;

// 互換性のため、bufferモジュールから必要な型を再エクスポート
pub use crate::buffer::{
    EditOperations, ChangeEvent, ChangeListener, CursorPosition,
    NavigationAction, NavigationError, NavigationPosition,
};
pub use crate::error::{EditError, Result};
