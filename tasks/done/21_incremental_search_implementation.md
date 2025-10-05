# インクリメンタル検索機能実装

## タスク概要
Emacsライクなインクリメンタル検索機能（C-s、C-r）の実装を行う。

## 目的
- リアルタイムで動作するインクリメンタル検索の実装
- 高速で応答性の高い検索システムの構築
- Emacsとの操作互換性確保

## 実装対象機能

### 基本機能
1. **前方インクリメンタル検索（C-s）**
   - カーソル位置から前方への検索
   - リアルタイムマッチ表示

2. **後方インクリメンタル検索（C-r）**
   - カーソル位置から後方への検索
   - リアルタイムマッチ表示

3. **検索語操作**
   - 文字入力による検索語の拡張
   - Backspaceによる検索語の短縮
   - C-s/C-rによる次/前候補への移動

### 検索制御機能
1. **検索終了・キャンセル**
   - Enter：現在位置で検索終了
   - C-g：検索キャンセル、元位置に復帰

2. **検索拡張機能**
   - C-w：カーソル位置の単語を検索語に追加
   - 検索の折り返し（ファイル末尾→先頭）

## モジュール構造

```
search/
├── mod.rs                    # 検索モジュール統合
├── incremental.rs           # インクリメンタル検索エンジン
├── state.rs                # 検索状態管理
├── matcher.rs              # 文字列マッチング
├── ui.rs                   # 検索UI統合
└── commands.rs             # 検索コマンド実装
```

## 実装詳細

### 検索エンジン実装
```rust
// src/search/incremental.rs

use crate::error::Result;
use crate::buffer::TextEditor;

/// インクリメンタル検索エンジン
pub struct IncrementalSearchEngine {
    /// 検索状態
    state: IncrementalSearchState,

    /// 文字列マッチャー
    matcher: Box<dyn StringMatcher>,

    /// 検索オプション
    options: SearchOptions,
}

impl IncrementalSearchEngine {
    pub fn new() -> Self {
        Self {
            state: IncrementalSearchState::new(),
            matcher: Box::new(BasicStringMatcher::new()),
            options: SearchOptions::default(),
        }
    }

    /// 検索開始
    pub fn start_search(&mut self, editor: &TextEditor, direction: SearchDirection) -> Result<()> {
        self.state = IncrementalSearchState::new();
        self.state.direction = direction;
        self.state.start_position = editor.cursor_position();
        self.state.current_position = self.state.start_position;
        self.state.is_active = true;

        Ok(())
    }

    /// 検索パターンに文字追加
    pub fn add_char(&mut self, ch: char, editor: &TextEditor) -> Result<SearchResult> {
        self.state.pattern.push(ch);
        self.update_search(editor)
    }

    /// 検索パターンから文字削除
    pub fn delete_char(&mut self, editor: &TextEditor) -> Result<SearchResult> {
        if self.state.pattern.is_empty() {
            return Ok(SearchResult::NoChange);
        }

        self.state.pattern.pop();
        if self.state.pattern.is_empty() {
            // パターンが空になった場合は開始位置に戻る
            self.state.current_position = self.state.start_position;
            return Ok(SearchResult::MovedTo(self.state.start_position));
        }

        self.update_search(editor)
    }

    /// 次の検索結果に移動
    pub fn move_to_next(&mut self, editor: &TextEditor) -> Result<SearchResult> {
        if self.state.pattern.is_empty() {
            return Ok(SearchResult::NoChange);
        }

        // 現在の位置から次のマッチを検索
        let search_start = match self.state.direction {
            SearchDirection::Forward => self.state.current_position + 1,
            SearchDirection::Backward => {
                if self.state.current_position == 0 {
                    editor.text_length()
                } else {
                    self.state.current_position - 1
                }
            }
        };

        self.find_next_match(editor, search_start)
    }

    /// 検索方向を反転
    pub fn reverse_direction(&mut self, editor: &TextEditor) -> Result<SearchResult> {
        self.state.direction = match self.state.direction {
            SearchDirection::Forward => SearchDirection::Backward,
            SearchDirection::Backward => SearchDirection::Forward,
        };

        self.move_to_next(editor)
    }

    /// 検索終了
    pub fn exit_search(&mut self) -> usize {
        self.state.is_active = false;
        self.state.current_position
    }

    /// 検索キャンセル
    pub fn cancel_search(&mut self) -> usize {
        self.state.is_active = false;
        self.state.start_position
    }

    /// 現在の検索状態取得
    pub fn state(&self) -> &IncrementalSearchState {
        &self.state
    }

    /// 検索実行・更新
    fn update_search(&mut self, editor: &TextEditor) -> Result<SearchResult> {
        if self.state.pattern.is_empty() {
            return Ok(SearchResult::NoChange);
        }

        self.find_next_match(editor, self.state.current_position)
    }

    /// 次のマッチを検索
    fn find_next_match(&mut self, editor: &TextEditor, start: usize) -> Result<SearchResult> {
        let text = editor.text();

        if let Some(match_pos) = self.matcher.find_next(
            text,
            &self.state.pattern,
            start,
            self.state.direction,
            self.options.case_sensitive,
        ) {
            self.state.current_position = match_pos.start;
            self.state.failed = false;
            Ok(SearchResult::MovedTo(match_pos.start))
        } else {
            // 折り返し検索を試行
            self.try_wrapped_search(editor)
        }
    }

    /// 折り返し検索
    fn try_wrapped_search(&mut self, editor: &TextEditor) -> Result<SearchResult> {
        let text = editor.text();
        let wrap_start = match self.state.direction {
            SearchDirection::Forward => 0,
            SearchDirection::Backward => text.len().saturating_sub(1),
        };

        if let Some(match_pos) = self.matcher.find_next(
            text,
            &self.state.pattern,
            wrap_start,
            self.state.direction,
            self.options.case_sensitive,
        ) {
            // 元の検索範囲を超えていないかチェック
            let in_original_range = match self.state.direction {
                SearchDirection::Forward => match_pos.start < self.state.start_position,
                SearchDirection::Backward => match_pos.start > self.state.start_position,
            };

            if in_original_range {
                self.state.current_position = match_pos.start;
                self.state.wrapped = true;
                self.state.failed = false;
                Ok(SearchResult::WrappedTo(match_pos.start))
            } else {
                self.state.failed = true;
                Ok(SearchResult::NotFound)
            }
        } else {
            self.state.failed = true;
            Ok(SearchResult::NotFound)
        }
    }
}

/// 検索結果
#[derive(Debug, Clone, PartialEq)]
pub enum SearchResult {
    /// 位置に移動
    MovedTo(usize),

    /// 折り返して位置に移動
    WrappedTo(usize),

    /// 見つからない
    NotFound,

    /// 変更なし
    NoChange,
}
```

### 検索状態管理
```rust
// src/search/state.rs

/// インクリメンタル検索状態
#[derive(Debug, Clone)]
pub struct IncrementalSearchState {
    /// 検索パターン
    pub pattern: String,

    /// 検索方向
    pub direction: SearchDirection,

    /// 現在のカーソル位置
    pub current_position: usize,

    /// 検索開始位置
    pub start_position: usize,

    /// 検索がアクティブか
    pub is_active: bool,

    /// 検索が失敗したか
    pub failed: bool,

    /// 検索が折り返したか
    pub wrapped: bool,
}

impl IncrementalSearchState {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
            direction: SearchDirection::Forward,
            current_position: 0,
            start_position: 0,
            is_active: false,
            failed: false,
            wrapped: false,
        }
    }

    /// 検索状態リセット
    pub fn reset(&mut self) {
        self.pattern.clear();
        self.is_active = false;
        self.failed = false;
        self.wrapped = false;
    }

    /// 検索が有効か
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.pattern.is_empty()
    }
}

/// 検索方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// 検索オプション
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// 大文字小文字を区別するか
    pub case_sensitive: bool,

    /// 単語境界で検索するか
    pub word_boundary: bool,

    /// 折り返し検索を許可するか
    pub allow_wrap: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false, // Emacs デフォルト
            word_boundary: false,
            allow_wrap: true,
        }
    }
}
```

### コマンド統合
```rust
// src/search/commands.rs

use crate::input::commands::{Command, CommandResult};
use super::{IncrementalSearchEngine, SearchDirection, SearchResult};

/// 検索関連コマンド
#[derive(Debug, Clone, PartialEq)]
pub enum SearchCommand {
    /// インクリメンタル検索開始
    StartIncrementalSearch(SearchDirection),

    /// 検索パターンに文字追加
    AddToSearchPattern(char),

    /// 検索パターンから文字削除
    DeleteFromSearchPattern,

    /// 次/前の検索結果に移動
    MoveToNextMatch,
    MoveToPreviousMatch,

    /// 検索方向反転
    ReverseSearchDirection,

    /// 検索終了
    ExitSearch,

    /// 検索キャンセル
    CancelSearch,

    /// カーソル位置の単語を検索パターンに追加
    AddWordAtCursor,
}

impl From<SearchCommand> for Command {
    fn from(search_cmd: SearchCommand) -> Self {
        match search_cmd {
            SearchCommand::StartIncrementalSearch(SearchDirection::Forward) =>
                Command::StartIncrementalSearchForward,
            SearchCommand::StartIncrementalSearch(SearchDirection::Backward) =>
                Command::StartIncrementalSearchBackward,
            // ... 他のコマンドマッピング
            _ => Command::Unknown(format!("SearchCommand::{:?}", search_cmd)),
        }
    }
}

/// 検索コマンド処理器
pub struct SearchCommandProcessor {
    /// 検索エンジン
    search_engine: IncrementalSearchEngine,
}

impl SearchCommandProcessor {
    pub fn new() -> Self {
        Self {
            search_engine: IncrementalSearchEngine::new(),
        }
    }

    /// 検索コマンド実行
    pub fn execute_search_command(
        &mut self,
        command: SearchCommand,
        editor: &mut crate::buffer::TextEditor,
    ) -> CommandResult {
        match command {
            SearchCommand::StartIncrementalSearch(direction) => {
                match self.search_engine.start_search(editor, direction) {
                    Ok(_) => CommandResult::success_with_message(
                        format!("{}検索を開始しました",
                            if direction == SearchDirection::Forward { "前方" } else { "後方" }
                        )
                    ),
                    Err(e) => CommandResult::error(format!("検索開始エラー: {}", e)),
                }
            },

            SearchCommand::AddToSearchPattern(ch) => {
                match self.search_engine.add_char(ch, editor) {
                    Ok(SearchResult::MovedTo(pos)) => {
                        editor.set_cursor_position(pos);
                        CommandResult::success()
                    },
                    Ok(SearchResult::WrappedTo(pos)) => {
                        editor.set_cursor_position(pos);
                        CommandResult::success_with_message("検索が折り返しました".to_string())
                    },
                    Ok(SearchResult::NotFound) => {
                        CommandResult::success_with_message("見つかりません".to_string())
                    },
                    Ok(SearchResult::NoChange) => CommandResult::success(),
                    Err(e) => CommandResult::error(format!("検索エラー: {}", e)),
                }
            },

            SearchCommand::DeleteFromSearchPattern => {
                match self.search_engine.delete_char(editor) {
                    Ok(SearchResult::MovedTo(pos)) => {
                        editor.set_cursor_position(pos);
                        CommandResult::success()
                    },
                    Ok(_) => CommandResult::success(),
                    Err(e) => CommandResult::error(format!("検索エラー: {}", e)),
                }
            },

            SearchCommand::MoveToNextMatch => {
                match self.search_engine.move_to_next(editor) {
                    Ok(SearchResult::MovedTo(pos)) => {
                        editor.set_cursor_position(pos);
                        CommandResult::success()
                    },
                    Ok(SearchResult::WrappedTo(pos)) => {
                        editor.set_cursor_position(pos);
                        CommandResult::success_with_message("検索が折り返しました".to_string())
                    },
                    Ok(SearchResult::NotFound) => {
                        CommandResult::success_with_message("見つかりません".to_string())
                    },
                    Ok(_) => CommandResult::success(),
                    Err(e) => CommandResult::error(format!("検索エラー: {}", e)),
                }
            },

            SearchCommand::ExitSearch => {
                let final_pos = self.search_engine.exit_search();
                editor.set_cursor_position(final_pos);
                CommandResult::success_with_message("検索を終了しました".to_string())
            },

            SearchCommand::CancelSearch => {
                let original_pos = self.search_engine.cancel_search();
                editor.set_cursor_position(original_pos);
                CommandResult::success_with_message("検索をキャンセルしました".to_string())
            },

            _ => CommandResult::error("未実装の検索コマンドです".to_string()),
        }
    }

    /// 検索状態の取得
    pub fn search_state(&self) -> &IncrementalSearchState {
        self.search_engine.state()
    }

    /// 検索がアクティブか
    pub fn is_search_active(&self) -> bool {
        self.search_engine.state().is_active
    }
}
```

## テスト実装

### 単体テスト
```rust
// src/search/tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::TextEditor;

    #[test]
    fn test_incremental_search_basic() {
        let mut engine = IncrementalSearchEngine::new();
        let mut editor = TextEditor::from_str("hello world hello");

        // 検索開始
        engine.start_search(&editor, SearchDirection::Forward).unwrap();

        // "hello" を検索
        let result = engine.add_char('h', &editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));

        let result = engine.add_char('e', &editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));

        let result = engine.add_char('l', &editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));

        let result = engine.add_char('l', &editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));

        let result = engine.add_char('o', &editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));

        // 次のマッチに移動
        let result = engine.move_to_next(&editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(12));
    }

    #[test]
    fn test_search_wrap_around() {
        let mut engine = IncrementalSearchEngine::new();
        let mut editor = TextEditor::from_str("world hello");
        editor.set_cursor_position(8); // "hello" の中

        engine.start_search(&editor, SearchDirection::Forward).unwrap();

        // "world" を検索（折り返し必要）
        let result = engine.add_char('w', &editor).unwrap();
        assert_eq!(result, SearchResult::WrappedTo(0));
        assert!(engine.state().wrapped);
    }

    #[test]
    fn test_search_not_found() {
        let mut engine = IncrementalSearchEngine::new();
        let editor = TextEditor::from_str("hello world");

        engine.start_search(&editor, SearchDirection::Forward).unwrap();

        // 存在しない文字列を検索
        let result = engine.add_char('z', &editor).unwrap();
        assert_eq!(result, SearchResult::NotFound);
        assert!(engine.state().failed);
    }

    #[test]
    fn test_search_backspace() {
        let mut engine = IncrementalSearchEngine::new();
        let editor = TextEditor::from_str("hello world");

        engine.start_search(&editor, SearchDirection::Forward).unwrap();

        // "he" を検索
        engine.add_char('h', &editor).unwrap();
        let result = engine.add_char('e', &editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));

        // "h" に戻る
        let result = engine.delete_char(&editor).unwrap();
        assert_eq!(result, SearchResult::MovedTo(0));
        assert_eq!(engine.state().pattern, "h");
    }
}
```

### 統合テスト
```rust
// tests/incremental_search_integration.rs

use altre::buffer::TextEditor;
use altre::input::commands::CommandProcessor;
use altre::search::SearchCommand;

#[test]
fn test_search_integration_with_commands() {
    let mut processor = CommandProcessor::new();
    processor.sync_editor_content("hello world hello universe");

    // C-s で検索開始
    let result = processor.execute(Command::StartIncrementalSearchForward);
    assert!(result.success);

    // 検索文字列入力
    let result = processor.execute(Command::InsertChar('h'));
    assert!(result.success);

    let result = processor.execute(Command::InsertChar('e'));
    assert!(result.success);

    // 次のマッチに移動
    let result = processor.execute(Command::MoveToNextMatch);
    assert!(result.success);

    // 検索終了
    let result = processor.execute(Command::ExitSearch);
    assert!(result.success);
}
```

## 依存関係
- 基本文字列検索アルゴリズム（前タスクで実装）
- キーバインドシステム
- ミニバッファシステム
- TextEditor統合

## 成果物
- インクリメンタル検索エンジン実装
- 検索状態管理システム
- 検索コマンド統合
- 包括的テストスイート

## 完了条件
- [ ] IncrementalSearchEngine実装完了
- [ ] 基本検索操作（C-s、C-r）動作確認
- [ ] 検索語操作（文字追加・削除）動作確認
- [ ] 検索制御（終了・キャンセル）動作確認
- [ ] 折り返し検索動作確認
- [ ] 単体・統合テスト実装完了
- [ ] 実際のTUIでの動作確認完了

## 進捗記録
- 作成日：2025-01-28
- 状態：実装準備完了