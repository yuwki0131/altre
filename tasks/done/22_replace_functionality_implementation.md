# 置換機能実装

## タスク概要
Emacsライクな置換機能（M-%、C-M-%）の実装を行う。インタラクティブな置換確認機能を含む。

## 目的
- クエリ置換によるインタラクティブな置換機能の実装
- 一括置換・個別確認・プレビュー機能の提供
- Emacs互換の操作体験の実現

## 実装対象機能

### 基本置換機能
1. **クエリ置換（M-%）**
   - 文字列の検索と置換確認
   - インタラクティブな置換操作

2. **置換操作キー**
   - `y` または `SPC`：置換して次へ
   - `n` または `DEL`：置換せずに次へ
   - `!`：残り全て置換
   - `q` または `Enter`：置換終了
   - `C-g`：置換キャンセル（全復元）

### 高度な置換機能
1. **正規表現置換（C-M-%）**
2. **条件付き置換**（将来実装）
3. **矩形置換**（将来実装）

## モジュール構造

```
search/
├── replace.rs              # 置換エンジン
├── replace_state.rs        # 置換状態管理
├── replace_ui.rs          # 置換UI統合
└── replace_commands.rs     # 置換コマンド実装
```

## 実装詳細

### 置換エンジン実装
```rust
// src/search/replace.rs

use crate::error::Result;
use crate::buffer::TextEditor;
use super::{SearchMatch, StringMatcher};

/// 置換エンジン
pub struct ReplaceEngine {
    /// 置換状態
    state: ReplaceState,

    /// 文字列マッチャー
    matcher: Box<dyn StringMatcher>,

    /// 置換履歴（アンドゥ用）
    history: ReplaceHistory,
}

impl ReplaceEngine {
    pub fn new() -> Self {
        Self {
            state: ReplaceState::new(),
            matcher: Box::new(BasicStringMatcher::new()),
            history: ReplaceHistory::new(),
        }
    }

    /// 置換開始
    pub fn start_replace(
        &mut self,
        editor: &TextEditor,
        search_pattern: String,
        replacement: String,
    ) -> Result<ReplaceResult> {
        // 全てのマッチを検索
        let matches = self.matcher.find_matches(
            editor.text(),
            &search_pattern,
            true, // case_sensitive
        );

        if matches.is_empty() {
            return Ok(ReplaceResult::NotFound);
        }

        self.state = ReplaceState::new();
        self.state.search_pattern = search_pattern;
        self.state.replacement = replacement;
        self.state.matches = matches;
        self.state.current_match_index = 0;
        self.state.is_active = true;
        self.state.start_position = editor.cursor_position();

        self.history.clear();

        Ok(ReplaceResult::Started {
            total_matches: self.state.matches.len(),
            first_match: self.state.matches[0].clone(),
        })
    }

    /// 現在の置換候補を置換
    pub fn replace_current(&mut self, editor: &mut TextEditor) -> Result<ReplaceResult> {
        if !self.state.is_valid() {
            return Ok(ReplaceResult::InvalidState);
        }

        let current_match = &self.state.matches[self.state.current_match_index];

        // 置換実行前の状態を保存（アンドゥ用）
        let undo_info = ReplaceUndoInfo {
            position: current_match.start,
            original_text: editor.text()[current_match.start..current_match.end].to_string(),
            replacement_text: self.state.replacement.clone(),
            match_index: self.state.current_match_index,
        };

        // 置換実行
        editor.replace_range(
            current_match.start,
            current_match.end,
            &self.state.replacement,
        )?;

        // 履歴に追加
        self.history.push(undo_info);

        // 置換による位置変更を計算
        let length_diff = self.state.replacement.len() as i32 -
                         (current_match.end - current_match.start) as i32;

        // 後続のマッチ位置を調整
        self.adjust_subsequent_matches(self.state.current_match_index, length_diff);

        // 統計更新
        self.state.replaced_count += 1;

        // 次のマッチに移動
        self.move_to_next_match(editor)
    }

    /// 現在の置換候補をスキップ
    pub fn skip_current(&mut self, editor: &mut TextEditor) -> Result<ReplaceResult> {
        if !self.state.is_valid() {
            return Ok(ReplaceResult::InvalidState);
        }

        self.state.skipped_count += 1;
        self.move_to_next_match(editor)
    }

    /// 残り全て置換
    pub fn replace_all(&mut self, editor: &mut TextEditor) -> Result<ReplaceResult> {
        if !self.state.is_valid() {
            return Ok(ReplaceResult::InvalidState);
        }

        let mut total_replaced = 0;
        let start_index = self.state.current_match_index;

        // 現在位置から最後まで全て置換
        while self.state.current_match_index < self.state.matches.len() {
            match self.replace_current(editor)? {
                ReplaceResult::Replaced { .. } => {
                    total_replaced += 1;
                    // replace_currentが次に移動するのでここでは何もしない
                },
                ReplaceResult::Finished => break,
                _ => break,
            }
        }

        Ok(ReplaceResult::AllReplaced {
            count: total_replaced
        })
    }

    /// 前の置換をアンドゥ
    pub fn undo_last(&mut self, editor: &mut TextEditor) -> Result<ReplaceResult> {
        if let Some(undo_info) = self.history.pop() {
            // 置換をアンドゥ
            let current_text = &editor.text()[undo_info.position..
                undo_info.position + undo_info.replacement_text.len()];

            if current_text == undo_info.replacement_text {
                // 元のテキストに復元
                editor.replace_range(
                    undo_info.position,
                    undo_info.position + undo_info.replacement_text.len(),
                    &undo_info.original_text,
                )?;

                // 位置調整
                let length_diff = undo_info.original_text.len() as i32 -
                                undo_info.replacement_text.len() as i32;
                self.adjust_subsequent_matches(undo_info.match_index, length_diff);

                // 統計更新
                self.state.replaced_count -= 1;
                self.state.current_match_index = undo_info.match_index;

                // カーソル位置調整
                editor.set_cursor_position(undo_info.position);

                Ok(ReplaceResult::Undone {
                    position: undo_info.position,
                })
            } else {
                Ok(ReplaceResult::UndoFailed {
                    reason: "テキストが変更されているためアンドゥできません".to_string(),
                })
            }
        } else {
            Ok(ReplaceResult::UndoFailed {
                reason: "アンドゥする置換操作がありません".to_string(),
            })
        }
    }

    /// 置換終了
    pub fn finish_replace(&mut self) -> ReplaceResult {
        self.state.is_active = false;

        ReplaceResult::Finished {
            total_replaced: self.state.replaced_count,
            total_skipped: self.state.skipped_count,
        }
    }

    /// 置換キャンセル
    pub fn cancel_replace(&mut self, editor: &mut TextEditor) -> Result<ReplaceResult> {
        // 全ての置換をアンドゥ
        let mut undone_count = 0;
        while !self.history.is_empty() {
            if let Ok(ReplaceResult::Undone { .. }) = self.undo_last(editor) {
                undone_count += 1;
            } else {
                break;
            }
        }

        // 元の位置に戻る
        editor.set_cursor_position(self.state.start_position);

        self.state.is_active = false;

        Ok(ReplaceResult::Cancelled {
            undone_count,
        })
    }

    /// 次のマッチに移動
    fn move_to_next_match(&mut self, editor: &mut TextEditor) -> Result<ReplaceResult> {
        self.state.current_match_index += 1;

        if self.state.current_match_index >= self.state.matches.len() {
            // 全て完了
            Ok(ReplaceResult::Finished {
                total_replaced: self.state.replaced_count,
                total_skipped: self.state.skipped_count,
            })
        } else {
            // 次のマッチにカーソル移動
            let next_match = &self.state.matches[self.state.current_match_index];
            editor.set_cursor_position(next_match.start);

            Ok(ReplaceResult::MovedToNext {
                match_index: self.state.current_match_index,
                total_matches: self.state.matches.len(),
                match_info: next_match.clone(),
            })
        }
    }

    /// 後続マッチの位置調整
    fn adjust_subsequent_matches(&mut self, from_index: usize, length_diff: i32) {
        for i in (from_index + 1)..self.state.matches.len() {
            let match_ref = &mut self.state.matches[i];
            if length_diff > 0 {
                match_ref.start += length_diff as usize;
                match_ref.end += length_diff as usize;
            } else if length_diff < 0 {
                let abs_diff = (-length_diff) as usize;
                match_ref.start = match_ref.start.saturating_sub(abs_diff);
                match_ref.end = match_ref.end.saturating_sub(abs_diff);
            }
        }
    }

    /// 現在の置換状態取得
    pub fn state(&self) -> &ReplaceState {
        &self.state
    }

    /// 現在のマッチ情報取得
    pub fn current_match(&self) -> Option<&SearchMatch> {
        if self.state.is_valid() && self.state.current_match_index < self.state.matches.len() {
            Some(&self.state.matches[self.state.current_match_index])
        } else {
            None
        }
    }
}

/// 置換結果
#[derive(Debug, Clone)]
pub enum ReplaceResult {
    /// 置換開始
    Started {
        total_matches: usize,
        first_match: SearchMatch,
    },

    /// 置換実行
    Replaced {
        position: usize,
        old_text: String,
        new_text: String,
    },

    /// 次のマッチに移動
    MovedToNext {
        match_index: usize,
        total_matches: usize,
        match_info: SearchMatch,
    },

    /// 全て置換
    AllReplaced {
        count: usize,
    },

    /// アンドゥ実行
    Undone {
        position: usize,
    },

    /// アンドゥ失敗
    UndoFailed {
        reason: String,
    },

    /// 置換完了
    Finished {
        total_replaced: usize,
        total_skipped: usize,
    },

    /// 置換キャンセル
    Cancelled {
        undone_count: usize,
    },

    /// マッチなし
    NotFound,

    /// 無効な状態
    InvalidState,
}
```

### 置換状態管理
```rust
// src/search/replace_state.rs

/// 置換状態
#[derive(Debug, Clone)]
pub struct ReplaceState {
    /// 検索パターン
    pub search_pattern: String,

    /// 置換文字列
    pub replacement: String,

    /// マッチリスト
    pub matches: Vec<SearchMatch>,

    /// 現在のマッチインデックス
    pub current_match_index: usize,

    /// 置換開始位置
    pub start_position: usize,

    /// 置換がアクティブか
    pub is_active: bool,

    /// 置換済み数
    pub replaced_count: usize,

    /// スキップ数
    pub skipped_count: usize,
}

impl ReplaceState {
    pub fn new() -> Self {
        Self {
            search_pattern: String::new(),
            replacement: String::new(),
            matches: Vec::new(),
            current_match_index: 0,
            start_position: 0,
            is_active: false,
            replaced_count: 0,
            skipped_count: 0,
        }
    }

    /// 置換状態が有効か
    pub fn is_valid(&self) -> bool {
        self.is_active &&
        !self.matches.is_empty() &&
        self.current_match_index < self.matches.len()
    }

    /// 進行状況計算
    pub fn progress(&self) -> ReplaceProgress {
        ReplaceProgress {
            current: self.current_match_index + 1,
            total: self.matches.len(),
            replaced: self.replaced_count,
            skipped: self.skipped_count,
        }
    }
}

/// 置換進行状況
#[derive(Debug, Clone)]
pub struct ReplaceProgress {
    pub current: usize,
    pub total: usize,
    pub replaced: usize,
    pub skipped: usize,
}

/// 置換アンドゥ情報
#[derive(Debug, Clone)]
pub struct ReplaceUndoInfo {
    /// 置換位置
    pub position: usize,

    /// 元のテキスト
    pub original_text: String,

    /// 置換後テキスト
    pub replacement_text: String,

    /// マッチインデックス
    pub match_index: usize,
}

/// 置換履歴（アンドゥ用）
pub struct ReplaceHistory {
    /// アンドゥ情報のスタック
    history: Vec<ReplaceUndoInfo>,

    /// 最大履歴数
    max_history: usize,
}

impl ReplaceHistory {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 100, // 最大100回のアンドゥを保持
        }
    }

    /// 履歴に追加
    pub fn push(&mut self, undo_info: ReplaceUndoInfo) {
        self.history.push(undo_info);

        // 履歴サイズ制限
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// 最新の履歴を取得して削除
    pub fn pop(&mut self) -> Option<ReplaceUndoInfo> {
        self.history.pop()
    }

    /// 履歴をクリア
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// 履歴が空か
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}
```

### 置換コマンド実装
```rust
// src/search/replace_commands.rs

use crate::input::commands::{Command, CommandResult};
use super::{ReplaceEngine, ReplaceResult};

/// 置換関連コマンド
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceCommand {
    /// クエリ置換開始
    StartQueryReplace {
        search_pattern: String,
        replacement: String,
    },

    /// 正規表現置換開始（将来実装）
    StartRegexReplace {
        pattern: String,
        replacement: String,
    },

    /// 現在のマッチを置換
    ReplaceCurrentMatch,

    /// 現在のマッチをスキップ
    SkipCurrentMatch,

    /// 残り全て置換
    ReplaceAllRemaining,

    /// 前の置換をアンドゥ
    UndoLastReplace,

    /// 置換終了
    FinishReplace,

    /// 置換キャンセル
    CancelReplace,
}

/// 置換コマンド処理器
pub struct ReplaceCommandProcessor {
    /// 置換エンジン
    replace_engine: ReplaceEngine,
}

impl ReplaceCommandProcessor {
    pub fn new() -> Self {
        Self {
            replace_engine: ReplaceEngine::new(),
        }
    }

    /// 置換コマンド実行
    pub fn execute_replace_command(
        &mut self,
        command: ReplaceCommand,
        editor: &mut crate::buffer::TextEditor,
    ) -> CommandResult {
        match command {
            ReplaceCommand::StartQueryReplace { search_pattern, replacement } => {
                match self.replace_engine.start_replace(editor, search_pattern.clone(), replacement.clone()) {
                    Ok(ReplaceResult::Started { total_matches, .. }) => {
                        CommandResult::success_with_message(
                            format!("'{}' を '{}' で置換します ({} 箇所)",
                                search_pattern, replacement, total_matches)
                        )
                    },
                    Ok(ReplaceResult::NotFound) => {
                        CommandResult::success_with_message(
                            format!("'{}' は見つかりませんでした", search_pattern)
                        )
                    },
                    Err(e) => CommandResult::error(format!("置換開始エラー: {}", e)),
                    _ => CommandResult::error("予期しない置換結果".to_string()),
                }
            },

            ReplaceCommand::ReplaceCurrentMatch => {
                match self.replace_engine.replace_current(editor) {
                    Ok(ReplaceResult::Replaced { .. }) => {
                        CommandResult::success_with_message("置換しました".to_string())
                    },
                    Ok(ReplaceResult::MovedToNext { match_index, total_matches, .. }) => {
                        CommandResult::success_with_message(
                            format!("置換しました ({}/{})", match_index + 1, total_matches)
                        )
                    },
                    Ok(ReplaceResult::Finished { total_replaced, total_skipped }) => {
                        CommandResult::success_with_message(
                            format!("置換完了: {} 箇所置換、{} 箇所スキップ",
                                total_replaced, total_skipped)
                        )
                    },
                    Ok(ReplaceResult::InvalidState) => {
                        CommandResult::error("置換状態が無効です".to_string())
                    },
                    Err(e) => CommandResult::error(format!("置換エラー: {}", e)),
                    _ => CommandResult::error("予期しない置換結果".to_string()),
                }
            },

            ReplaceCommand::SkipCurrentMatch => {
                match self.replace_engine.skip_current(editor) {
                    Ok(ReplaceResult::MovedToNext { match_index, total_matches, .. }) => {
                        CommandResult::success_with_message(
                            format!("スキップしました ({}/{})", match_index + 1, total_matches)
                        )
                    },
                    Ok(ReplaceResult::Finished { total_replaced, total_skipped }) => {
                        CommandResult::success_with_message(
                            format!("置換完了: {} 箇所置換、{} 箇所スキップ",
                                total_replaced, total_skipped)
                        )
                    },
                    Ok(ReplaceResult::InvalidState) => {
                        CommandResult::error("置換状態が無効です".to_string())
                    },
                    Err(e) => CommandResult::error(format!("スキップエラー: {}", e)),
                    _ => CommandResult::error("予期しない置換結果".to_string()),
                }
            },

            ReplaceCommand::ReplaceAllRemaining => {
                match self.replace_engine.replace_all(editor) {
                    Ok(ReplaceResult::AllReplaced { count }) => {
                        CommandResult::success_with_message(
                            format!("残り {} 箇所を全て置換しました", count)
                        )
                    },
                    Ok(ReplaceResult::InvalidState) => {
                        CommandResult::error("置換状態が無効です".to_string())
                    },
                    Err(e) => CommandResult::error(format!("一括置換エラー: {}", e)),
                    _ => CommandResult::error("予期しない置換結果".to_string()),
                }
            },

            ReplaceCommand::UndoLastReplace => {
                match self.replace_engine.undo_last(editor) {
                    Ok(ReplaceResult::Undone { .. }) => {
                        CommandResult::success_with_message("前の置換をアンドゥしました".to_string())
                    },
                    Ok(ReplaceResult::UndoFailed { reason }) => {
                        CommandResult::error(format!("アンドゥ失敗: {}", reason))
                    },
                    Err(e) => CommandResult::error(format!("アンドゥエラー: {}", e)),
                    _ => CommandResult::error("予期しない置換結果".to_string()),
                }
            },

            ReplaceCommand::FinishReplace => {
                let result = self.replace_engine.finish_replace();
                match result {
                    ReplaceResult::Finished { total_replaced, total_skipped } => {
                        CommandResult::success_with_message(
                            format!("置換終了: {} 箇所置換、{} 箇所スキップ",
                                total_replaced, total_skipped)
                        )
                    },
                    _ => CommandResult::success_with_message("置換を終了しました".to_string()),
                }
            },

            ReplaceCommand::CancelReplace => {
                match self.replace_engine.cancel_replace(editor) {
                    Ok(ReplaceResult::Cancelled { undone_count }) => {
                        CommandResult::success_with_message(
                            format!("置換をキャンセルしました ({} 箇所を元に戻しました)",
                                undone_count)
                        )
                    },
                    Err(e) => CommandResult::error(format!("キャンセルエラー: {}", e)),
                    _ => CommandResult::error("予期しないキャンセル結果".to_string()),
                }
            },

            _ => CommandResult::error("未実装の置換コマンドです".to_string()),
        }
    }

    /// 置換状態の取得
    pub fn replace_state(&self) -> &ReplaceState {
        self.replace_engine.state()
    }

    /// 現在のマッチ情報取得
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.replace_engine.current_match()
    }

    /// 置換がアクティブか
    pub fn is_replace_active(&self) -> bool {
        self.replace_engine.state().is_active
    }
}
```

## テスト実装

### 単体テスト
```rust
// src/search/replace_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::TextEditor;

    #[test]
    fn test_basic_replace() {
        let mut engine = ReplaceEngine::new();
        let mut editor = TextEditor::from_str("hello world hello");

        // 置換開始
        let result = engine.start_replace(
            &editor,
            "hello".to_string(),
            "hi".to_string(),
        ).unwrap();

        match result {
            ReplaceResult::Started { total_matches, .. } => {
                assert_eq!(total_matches, 2);
            },
            _ => panic!("Expected Started result"),
        }

        // 最初のマッチを置換
        let result = engine.replace_current(&mut editor).unwrap();
        assert!(matches!(result, ReplaceResult::MovedToNext { .. }));
        assert_eq!(editor.text(), "hi world hello");

        // 2番目のマッチを置換
        let result = engine.replace_current(&mut editor).unwrap();
        assert!(matches!(result, ReplaceResult::Finished { .. }));
        assert_eq!(editor.text(), "hi world hi");
    }

    #[test]
    fn test_replace_undo() {
        let mut engine = ReplaceEngine::new();
        let mut editor = TextEditor::from_str("hello world");

        engine.start_replace(&editor, "hello".to_string(), "hi".to_string()).unwrap();
        engine.replace_current(&mut editor).unwrap();
        assert_eq!(editor.text(), "hi world");

        // アンドゥ
        let result = engine.undo_last(&mut editor).unwrap();
        assert!(matches!(result, ReplaceResult::Undone { .. }));
        assert_eq!(editor.text(), "hello world");
    }

    #[test]
    fn test_replace_all() {
        let mut engine = ReplaceEngine::new();
        let mut editor = TextEditor::from_str("hello world hello universe hello");

        engine.start_replace(&editor, "hello".to_string(), "hi".to_string()).unwrap();

        let result = engine.replace_all(&mut editor).unwrap();
        assert!(matches!(result, ReplaceResult::AllReplaced { count: 3 }));
        assert_eq!(editor.text(), "hi world hi universe hi");
    }

    #[test]
    fn test_replace_cancel() {
        let mut engine = ReplaceEngine::new();
        let mut editor = TextEditor::from_str("hello world hello");
        let original_text = editor.text().to_string();

        engine.start_replace(&editor, "hello".to_string(), "hi".to_string()).unwrap();
        engine.replace_current(&mut editor).unwrap(); // 1回置換

        // キャンセル（全て元に戻す）
        let result = engine.cancel_replace(&mut editor).unwrap();
        assert!(matches!(result, ReplaceResult::Cancelled { .. }));
        assert_eq!(editor.text(), original_text);
    }
}
```

## 依存関係
- インクリメンタル検索システム
- 文字列検索・マッチングアルゴリズム
- TextEditorのrange操作
- ミニバッファシステム

## 成果物
- 置換エンジン実装
- 置換状態管理システム
- 置換コマンド統合
- アンドゥ・リドゥ機能
- 包括的テストスイート

## 完了条件
- [x] 置換エンジン実装完了（`src/search/replace.rs:1`）
- [x] 基本置換操作（`M-%`）動作確認（`src/app.rs:632` でキー処理）
- [x] 置換制御キー（y/n/!/q/C-g）動作確認（`src/app.rs:689` 付近）
- [x] アンドゥ機能動作確認（`QueryReplaceController::cancel` で復元）
- [x] 一括置換機能動作確認（`accept_all` 実装）
- [x] 単体・統合テスト実装完了（`src/search/replace.rs:420` / `tests/search_replace_workflow.rs:1`）
- [x] ミニバッファ連携による TUI 動作確認（`src/minibuffer/system.rs:232` / `src/app.rs:1189`）

## 実施ログ
- 2025-02-05: `QueryReplaceController` を実装し、リテラル／正規表現両対応の候補生成を追加。
- 2025-02-05: `src/app.rs` に置換用キー処理を実装し、ミニバッファから `query-replace` / `query-replace-regexp` を起動可能にした。
- 2025-02-05: `search_replace_workflow.rs` を追加し、操作フローを統合テストで検証。

## ステータス
- `src/search/` には現状 `matcher.rs` など検索専用モジュールのみで置換エンジンが未実装（2025-02-05 時点）。
- ミニバッファからの置換コマンドも未登録であり、`src/input/commands.rs:1` に対応するコマンド定義が存在しない。
- テストスケルトンも作成されていないため、実装着手前にモジュール分割と API の再調整が必要。

## 次アクション
1. `src/search/` 配下に `replace.rs` など予定モジュールを追加し、`ReplaceEngine` と状態管理を実装。
2. `src/input/commands.rs` および `src/minibuffer/system.rs` に置換コマンドを統合し、キーシーケンス `M-%` を登録。
3. `tests/` へ置換機能の単体・統合テストを追加し、Undo/Redo 連携を検証。
