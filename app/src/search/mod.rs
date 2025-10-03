//! 検索モジュール
//!
//! インクリメンタル検索の状態管理とUI連携を提供

mod matcher;
mod state;
pub mod types;

use crate::buffer::TextEditor;
use matcher::{LiteralMatcher, StringMatcher};
use state::SearchState;
use types::SearchMatch;

pub use types::{HighlightKind, SearchDirection, SearchHighlight, SearchStatus, SearchUiState};

/// 検索制御インターフェース
#[derive(Debug)]
pub struct SearchController<M: StringMatcher = LiteralMatcher> {
    matcher: M,
    state: SearchState,
    last_pattern: Option<String>,
    case_sensitive: bool,
    highlights: Vec<SearchHighlight>,
    ui_state: Option<SearchUiState>,
    text_cache: String,
}

impl SearchController<LiteralMatcher> {
    /// 既定のリテラルマッチャーで作成
    pub fn new() -> Self {
        Self::with_matcher(LiteralMatcher::new())
    }
}

impl<M: StringMatcher> SearchController<M> {
    /// マッチャーを差し替えて作成
    pub fn with_matcher(matcher: M) -> Self {
        Self {
            matcher,
            state: SearchState::new(),
            last_pattern: None,
            case_sensitive: true,
            highlights: Vec::new(),
            ui_state: None,
            text_cache: String::new(),
        }
    }

    /// 検索がアクティブか判定
    pub fn is_active(&self) -> bool {
        self.state.active
    }

    /// 現在のUI状態を取得
    pub fn ui_state(&self) -> Option<&SearchUiState> {
        self.ui_state.as_ref()
    }

    /// ハイライト情報を取得
    pub fn highlights(&self) -> &[SearchHighlight] {
        &self.highlights
    }

    /// 検索を開始
    pub fn start(&mut self, editor: &mut TextEditor, direction: SearchDirection) {
        let cursor = *editor.cursor();
        self.state.reset();
        self.state.active = true;
        self.state.direction = direction;
        self.state.start_cursor = Some(cursor);
        self.state.start_char_index = cursor.char_pos;
        self.state.pattern = self.last_pattern.clone().unwrap_or_default();
        self.update_case_sensitivity();
        self.state.failed = false;

        if !self.state.pattern.is_empty() {
            let text = editor.to_string();
            self.recompute_matches(&text);
            self.select_match_near_cursor(editor, cursor.char_pos);
        } else {
            self.update_ui_state();
        }
    }

    /// 文字を追加
    pub fn input_char(&mut self, editor: &mut TextEditor, ch: char) {
        if !self.state.active {
            self.start(editor, SearchDirection::Forward);
        }

        self.state.pattern.push(ch);
        self.update_case_sensitivity();
        let text = editor.to_string();
        self.recompute_matches(&text);
        self.select_match_near_cursor(editor, self.state.start_char_index);
    }

    /// 検索語を1文字削除
    pub fn delete_char(&mut self, editor: &mut TextEditor) {
        if !self.state.active || self.state.pattern.is_empty() {
            self.state.pattern.clear();
            self.state.matches.clear();
            self.highlights.clear();
            self.state.current_index = None;
            self.state.failed = false;
            self.update_ui_state();
            return;
        }

        self.state.pattern.pop();
        self.update_case_sensitivity();
        let text = editor.to_string();
        if self.state.pattern.is_empty() {
            self.state.matches.clear();
            self.highlights.clear();
            self.state.current_index = None;
            self.state.failed = false;
            self.state.wrapped = false;
            self.update_ui_state();
            if let Some(start) = self.state.start_cursor {
                let _ = editor.move_cursor_to_char(start.char_pos);
            }
            return;
        }

        self.recompute_matches(&text);
        self.select_match_near_cursor(editor, self.state.start_char_index);
    }

    /// 次のマッチへ移動（前方検索）
    pub fn repeat_forward(&mut self, editor: &mut TextEditor) {
        self.repeat(editor, SearchDirection::Forward);
    }

    /// 前のマッチへ移動（後方検索）
    pub fn repeat_backward(&mut self, editor: &mut TextEditor) {
        self.repeat(editor, SearchDirection::Backward);
    }

    /// カーソル位置の単語を検索語に追加
    pub fn add_word_at_cursor(&mut self, editor: &mut TextEditor) {
        let word = self.extract_word_at_cursor(editor);
        if word.is_empty() {
            self.state.failed = true;
            self.update_ui_state();
            return;
        }

        if !self.state.pattern.is_empty() {
            self.state.pattern.push(' ');
        }
        self.state.pattern.push_str(&word);
        self.update_case_sensitivity();
        let text = editor.to_string();
        self.recompute_matches(&text);
        self.select_match_near_cursor(editor, self.state.start_char_index);
    }

    /// 検索を確定
    pub fn accept(&mut self) {
        if !self.state.active {
            return;
        }
        if !self.state.pattern.is_empty() {
            self.last_pattern = Some(self.state.pattern.clone());
        }
        self.state.active = false;
        self.ui_state = None;
        self.highlights.clear();
    }

    /// 検索をキャンセルし、カーソルを戻す
    pub fn cancel(&mut self, editor: &mut TextEditor) {
        if let Some(start) = self.state.start_cursor {
            let _ = editor.move_cursor_to_char(start.char_pos);
        }
        self.state.reset();
        self.highlights.clear();
        self.ui_state = None;
    }

    fn repeat(&mut self, editor: &mut TextEditor, dir: SearchDirection) {
        if !self.state.active {
            self.start(editor, dir);
            return;
        }

        if self.state.pattern.is_empty() {
            self.update_ui_state();
            return;
        }

        if self.state.matches.is_empty() {
            // パターンはあるが未マッチ
            self.state.failed = true;
            self.update_ui_state();
            return;
        }

        let next_index = match (self.state.current_index, dir) {
            (Some(idx), SearchDirection::Forward) if idx + 1 < self.state.matches.len() => {
                self.state.wrapped = false;
                idx + 1
            }
            (Some(_), SearchDirection::Forward) | (None, SearchDirection::Forward) => {
                self.state.wrapped = true;
                0
            }
            (Some(idx), SearchDirection::Backward) if idx > 0 => {
                self.state.wrapped = false;
                idx - 1
            }
            (Some(_), SearchDirection::Backward) | (None, SearchDirection::Backward) => {
                self.state.wrapped = true;
                self.state.matches.len() - 1
            }
        };

        self.state.direction = dir;
        self.move_to_match(editor, next_index);
    }

    fn select_match_near_cursor(&mut self, editor: &mut TextEditor, start_char: usize) {
        if self.state.matches.is_empty() {
            self.state.current_index = None;
            self.state.failed = true;
            self.highlights.clear();
            self.update_ui_state();
            return;
        }

        let idx = match self.state.direction {
            SearchDirection::Forward => self
                .state
                .matches
                .iter()
                .enumerate()
                .find(|(_, m)| m.start >= start_char)
                .map(|(idx, _)| idx)
                .or_else(|| {
                    self.state.wrapped = true;
                    Some(0)
                }),
            SearchDirection::Backward => self
                .state
                .matches
                .iter()
                .enumerate()
                .rev()
                .find(|(_, m)| m.start <= start_char)
                .map(|(idx, _)| idx)
                .or_else(|| {
                    self.state.wrapped = true;
                    Some(self.state.matches.len() - 1)
                }),
        };

        if let Some(idx) = idx {
            self.move_to_match(editor, idx);
        } else {
            self.state.current_index = None;
            self.state.failed = true;
            self.highlights.clear();
            self.update_ui_state();
        }
    }

    fn move_to_match(&mut self, editor: &mut TextEditor, index: usize) {
        if let Some(m) = self.state.matches.get(index).cloned() {
            self.state.current_index = Some(index);
            self.state.failed = false;
            let _ = editor.move_cursor_to_char(m.start);
            self.rebuild_highlights();
            self.update_ui_state();
        }
    }

    fn recompute_matches(&mut self, text: &str) {
        self.text_cache = text.to_string();
        self.state.matches = self
            .matcher
            .find_matches(text, &self.state.pattern, self.case_sensitive);
        self.state.current_index = None;
        self.state.failed = self.state.matches.is_empty();
        self.state.wrapped = false;
        self.rebuild_highlights();
        self.update_ui_state();
    }

    fn rebuild_highlights(&mut self) {
        self.highlights.clear();
        if self.state.pattern.is_empty() {
            return;
        }

        for (idx, m) in self.state.matches.iter().enumerate() {
            let span_len = self.highlight_span(m);
            if span_len == 0 {
                continue;
            }
            self.highlights.push(SearchHighlight {
                line: m.line,
                start_column: m.column,
                end_column: m.column + span_len,
                is_current: Some(idx) == self.state.current_index,
                kind: HighlightKind::Search,
            });
        }
    }

    fn highlight_span(&self, m: &SearchMatch) -> usize {
        let mut count = 0usize;
        for ch in self
            .text_cache
            .chars()
            .skip(m.start)
            .take(m.len())
        {
            if ch == '\n' {
                break;
            }
            count += 1;
        }
        count
    }

    fn update_ui_state(&mut self) {
        if !self.state.active {
            self.ui_state = None;
            return;
        }

        let status = if self.state.failed {
            SearchStatus::NotFound
        } else if self.state.wrapped {
            SearchStatus::Wrapped
        } else {
            SearchStatus::Active
        };

        let current = self.state.current_index.map(|idx| idx + 1);
        let message = if self.state.failed {
            Some(format!("{} は見つかりません", self.state.pattern))
        } else if self.state.wrapped {
            Some("検索が折り返しました".to_string())
        } else {
            None
        };

        self.ui_state = Some(SearchUiState {
            prompt_label: self.state.direction.label().to_string(),
            pattern: self.state.pattern.clone(),
            status,
            current_match: current,
            total_matches: self.state.matches.len(),
            wrapped: self.state.wrapped,
            message,
            direction: self.state.direction,
        });
    }

    fn update_case_sensitivity(&mut self) {
        let has_upper = self.state.pattern.chars().any(|c| c.is_uppercase());
        self.case_sensitive = has_upper;
    }

    fn extract_word_at_cursor(&self, editor: &TextEditor) -> String {
        let text = editor.to_string();
        let cursor = editor.cursor();
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return String::new();
        }

        let mut start = cursor.char_pos.min(chars.len());
        let mut end = start;

        // バックスキャン
        while start > 0 {
            if chars[start - 1].is_alphanumeric() || chars[start - 1] == '_' {
                start -= 1;
            } else {
                break;
            }
        }

        // フォワードスキャン
        while end < chars.len() {
            if chars[end].is_alphanumeric() || chars[end] == '_' {
                end += 1;
            } else {
                break;
            }
        }

        chars[start..end].iter().collect()
    }
}

// ジェネリックに対するデフォルト実装
impl Default for SearchController<LiteralMatcher> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{SearchController, SearchDirection};
    use crate::buffer::TextEditor;

    #[test]
    fn incremental_search_moves_cursor_forward() {
        let mut editor = TextEditor::from_str("hello world hello");
        let mut controller = SearchController::new();

        controller.start(&mut editor, SearchDirection::Forward);
        controller.input_char(&mut editor, 'w');

        assert_eq!(editor.cursor().char_pos, 6);
        let ui = controller.ui_state().expect("ui state");
        assert_eq!(ui.current_match, Some(1));
        assert_eq!(ui.total_matches, 1);
    }

    #[test]
    fn repeat_forward_wraps() {
        let mut editor = TextEditor::from_str("foo foo");
        let mut controller = SearchController::new();

        controller.start(&mut editor, SearchDirection::Forward);
        controller.input_char(&mut editor, 'f');
        controller.input_char(&mut editor, 'o');
        controller.input_char(&mut editor, 'o');

        controller.repeat_forward(&mut editor);
        assert_eq!(editor.cursor().char_pos, 4);

        controller.repeat_forward(&mut editor);
        assert_eq!(editor.cursor().char_pos, 0);
        let ui = controller.ui_state().expect("ui state");
        assert!(ui.wrapped);
    }

    #[test]
    fn delete_char_resets_when_empty() {
        let mut editor = TextEditor::from_str("abc");
        let mut controller = SearchController::new();

        controller.start(&mut editor, SearchDirection::Forward);
        controller.input_char(&mut editor, 'a');
        controller.delete_char(&mut editor);

        assert!(controller.highlights().is_empty());
        let ui = controller.ui_state().expect("ui state");
        assert_eq!(ui.pattern, "");
    }
}
