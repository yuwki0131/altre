//! インクリメンタル検索の状態管理

use crate::buffer::CursorPosition;
use super::types::{SearchDirection, SearchMatch};

/// インクリメンタル検索の内部状態
#[derive(Debug, Clone)]
pub struct SearchState {
    /// 検索がアクティブか
    pub active: bool,
    /// 検索パターン
    pub pattern: String,
    /// 検索方向
    pub direction: SearchDirection,
    /// マッチ集合
    pub matches: Vec<SearchMatch>,
    /// 現在選択されているマッチ
    pub current_index: Option<usize>,
    /// 折り返しが発生したか
    pub wrapped: bool,
    /// マッチ失敗状態
    pub failed: bool,
    /// 検索開始時のカーソル位置
    pub start_cursor: Option<CursorPosition>,
    /// 検索開始時のカーソル位置（文字インデックス）
    pub start_char_index: usize,
}

impl SearchState {
    /// 新しい状態を作成
    pub fn new() -> Self {
        Self {
            active: false,
            pattern: String::new(),
            direction: SearchDirection::Forward,
            matches: Vec::new(),
            current_index: None,
            wrapped: false,
            failed: false,
            start_cursor: None,
            start_char_index: 0,
        }
    }

    /// 状態をリセット
    pub fn reset(&mut self) {
        self.active = false;
        self.pattern.clear();
        self.direction = SearchDirection::Forward;
        self.matches.clear();
        self.current_index = None;
        self.wrapped = false;
        self.failed = false;
        self.start_cursor = None;
        self.start_char_index = 0;
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
