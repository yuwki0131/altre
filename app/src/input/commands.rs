//! コマンドシステム
//!
//! エディタコマンドの定義と実行

use crate::buffer::{EditOperations, NavigationAction, TextEditor};
use crate::editor::KillRing;
use crate::file::{FileOperationManager, FileBuffer, expand_path};
/// コマンド実行の結果
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// 実行が成功したか
    pub success: bool,
    /// 結果メッセージ
    pub message: Option<String>,
    /// 画面更新が必要か
    pub needs_refresh: bool,
    /// アプリケーションを終了するか
    pub should_quit: bool,
}

impl CommandResult {
    /// 成功結果を作成
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            needs_refresh: true,
            should_quit: false,
        }
    }

    /// メッセージ付き成功結果を作成
    pub fn success_with_message(message: String) -> Self {
        Self {
            success: true,
            message: Some(message),
            needs_refresh: true,
            should_quit: false,
        }
    }

    /// エラー結果を作成
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message: Some(message),
            needs_refresh: false,
            should_quit: false,
        }
    }

    /// 終了結果を作成
    pub fn quit() -> Self {
        Self {
            success: true,
            message: Some("アプリケーションを終了します".to_string()),
            needs_refresh: false,
            should_quit: true,
        }
    }

    /// 画面更新なしの成功結果を作成
    pub fn success_no_refresh() -> Self {
        Self {
            success: true,
            message: None,
            needs_refresh: false,
            should_quit: false,
        }
    }
}

/// キルの追記方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillJoin {
    /// 既存エントリ末尾に連結
    Append,
    /// 既存エントリ先頭に連結
    Prepend,
}

/// 直前のコマンド種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LastCommand {
    Other,
    Kill,
    Yank,
}

/// コマンドの種類
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // カーソル移動
    ForwardChar,
    BackwardChar,
    NextLine,
    PreviousLine,
    ForwardWord,
    BackwardWord,

    // 編集操作
    InsertChar(char),
    DeleteBackwardChar,
    DeleteChar,
    InsertNewline,
    KillWordForward,
    KillWordBackward,
    KillLine,
    Yank,
    YankPop,
    KeyboardQuit,
    Undo,
    Redo,
    SetMark,
    KillRegion,
    CopyRegion,
    ExchangePointAndMark,
    MarkBuffer,
    ScrollPageDown,
    ScrollPageUp,
    Recenter,
    ScrollLeft,
    ScrollRight,

    // ファイル操作
    FindFile,
    SaveBuffer,
    WriteFile,        // C-x C-w (別名保存)
    SaveAllBuffers,   // C-x s (全バッファ保存)

    // バッファ操作
    SwitchToBuffer,   // C-x b
    KillBuffer,       // C-x k
    ListBuffers,      // C-x C-b

    // ウィンドウ操作
    SplitWindowBelow,   // C-x 2
    SplitWindowRight,   // C-x 3
    DeleteOtherWindows, // C-x 1
    DeleteWindow,       // C-x 0
    OtherWindow,        // C-x o

    // アプリケーション制御
    SaveBuffersKillTerminal,
    Quit,
    ExecuteCommand,
    MoveLineStart,
    MoveLineEnd,
    MoveBufferStart,
    MoveBufferEnd,
    EvalExpression,
    QueryReplace,
    RegexQueryReplace,

    // 未知のコマンド
    Unknown(String),
}

impl Command {
    /// 文字列からコマンドを作成
    pub fn from_string(cmd: &str) -> Self {
        match cmd {
            "forward-char" => Command::ForwardChar,
            "backward-char" => Command::BackwardChar,
            "next-line" => Command::NextLine,
            "previous-line" => Command::PreviousLine,
            "forward-word" => Command::ForwardWord,
            "backward-word" => Command::BackwardWord,
            "delete-backward-char" => Command::DeleteBackwardChar,
            "delete-char" => Command::DeleteChar,
            "newline" => Command::InsertNewline,
            "kill-word" => Command::KillWordForward,
            "backward-kill-word" => Command::KillWordBackward,
            "kill-line" => Command::KillLine,
            "yank" => Command::Yank,
            "yank-pop" => Command::YankPop,
            "keyboard-quit" => Command::KeyboardQuit,
            "undo" => Command::Undo,
            "redo" => Command::Redo,
            "set-mark-command" => Command::SetMark,
            "kill-region" => Command::KillRegion,
            "copy-region-as-kill" => Command::CopyRegion,
            "exchange-point-and-mark" => Command::ExchangePointAndMark,
            "mark-whole-buffer" => Command::MarkBuffer,
            "scroll-up" => Command::ScrollPageDown,
            "scroll-down" => Command::ScrollPageUp,
            "recenter-top-bottom" => Command::Recenter,
            "scroll-left" => Command::ScrollLeft,
            "scroll-right" => Command::ScrollRight,
            "find-file" => Command::FindFile,
            "save-buffer" => Command::SaveBuffer,
            "write-file" => Command::WriteFile,
            "save-some-buffers" => Command::SaveAllBuffers,
            "switch-to-buffer" => Command::SwitchToBuffer,
            "kill-buffer" => Command::KillBuffer,
            "list-buffers" => Command::ListBuffers,
            "split-window-below" => Command::SplitWindowBelow,
            "split-window-right" => Command::SplitWindowRight,
            "delete-other-windows" => Command::DeleteOtherWindows,
            "delete-window" => Command::DeleteWindow,
            "other-window" => Command::OtherWindow,
            "save-buffers-kill-terminal" => Command::SaveBuffersKillTerminal,
            "quit" => Command::Quit,
            "execute-command" => Command::ExecuteCommand,
            "eval-expression" => Command::EvalExpression,
            "move-beginning-of-line" => Command::MoveLineStart,
            "move-end-of-line" => Command::MoveLineEnd,
            "beginning-of-buffer" => Command::MoveBufferStart,
            "end-of-buffer" => Command::MoveBufferEnd,
            "query-replace" => Command::QueryReplace,
            "query-replace-regexp" => Command::RegexQueryReplace,
            _ => Command::Unknown(cmd.to_string()),
        }
    }

    /// コマンドの説明を取得
    pub fn description(&self) -> &'static str {
        match self {
            Command::ForwardChar => "カーソルを右に移動",
            Command::BackwardChar => "カーソルを左に移動",
            Command::NextLine => "カーソルを下に移動",
            Command::PreviousLine => "カーソルを上に移動",
            Command::ForwardWord => "次の単語末尾に移動",
            Command::BackwardWord => "前の単語先頭に移動",
            Command::InsertChar(_) => "文字を挿入",
            Command::DeleteBackwardChar => "前の文字を削除",
            Command::DeleteChar => "カーソル位置の文字を削除",
            Command::KillWordForward => "次の単語を削除",
            Command::KillWordBackward => "前の単語を削除",
            Command::KillLine => "行末まで削除",
            Command::Yank => "キルリングから貼り付け",
            Command::YankPop => "直前のヤンクを置き換え",
            Command::KeyboardQuit => "操作をキャンセル",
            Command::Undo => "直前の操作を取り消し",
            Command::Redo => "取り消した操作をやり直し",
            Command::SetMark => "マークを設定",
            Command::KillRegion => "リージョンを削除",
            Command::CopyRegion => "リージョンをコピー",
            Command::ExchangePointAndMark => "カーソルとマークを交換",
            Command::MarkBuffer => "バッファ全体を選択",
            Command::ScrollPageDown => "画面を下にスクロール",
            Command::ScrollPageUp => "画面を上にスクロール",
            Command::Recenter => "画面を再配置",
            Command::ScrollLeft => "画面を左にスクロール",
            Command::ScrollRight => "画面を右にスクロール",
            Command::InsertNewline => "改行を挿入",
            Command::FindFile => "ファイルを開く",
            Command::SaveBuffer => "バッファを保存",
            Command::WriteFile => "別名でファイルを保存",
            Command::SaveAllBuffers => "すべてのバッファを保存",
            Command::SwitchToBuffer => "バッファを切り替え",
            Command::KillBuffer => "バッファを削除",
            Command::ListBuffers => "バッファ一覧を表示",
            Command::SplitWindowBelow => "ウィンドウを上下に分割",
            Command::SplitWindowRight => "ウィンドウを左右に分割",
            Command::DeleteOtherWindows => "現在のウィンドウのみ表示",
            Command::DeleteWindow => "現在のウィンドウを閉じる",
            Command::OtherWindow => "次のウィンドウに移動",
            Command::SaveBuffersKillTerminal => "保存して終了",
            Command::Quit => "終了",
            Command::ExecuteCommand => "コマンドを実行",
            Command::EvalExpression => "式を評価",
            Command::MoveLineStart => "行頭に移動",
            Command::MoveLineEnd => "行末に移動",
            Command::MoveBufferStart => "バッファ先頭に移動",
            Command::MoveBufferEnd => "バッファ末尾に移動",
            Command::QueryReplace => "クエリ置換を実行",
            Command::RegexQueryReplace => "正規表現クエリ置換を実行",
            Command::Unknown(_) => "不明なコマンド",
        }
    }
}

/// コマンド処理器
pub struct CommandProcessor {
    editor: TextEditor,
    file_manager: FileOperationManager,
    current_buffer: Option<FileBuffer>,
    kill_ring: KillRing,
    last_command: LastCommand,
    last_yank_range: Option<(usize, usize)>,
}

impl CommandProcessor {
    /// 新しいコマンド処理器を作成
    pub fn new() -> Self {
        Self {
            editor: TextEditor::new(),
            file_manager: FileOperationManager::new(),
            current_buffer: None,
            kill_ring: KillRing::new(),
            last_command: LastCommand::Other,
            last_yank_range: None,
        }
    }

    /// エディタへの参照（テスト用途）
    pub fn editor(&self) -> &TextEditor {
        &self.editor
    }

    /// 現在のバッファへの参照
    pub fn current_buffer(&self) -> Option<&FileBuffer> {
        self.current_buffer.as_ref()
    }

    /// 現在のバッファへの可変参照
    pub fn current_buffer_mut(&mut self) -> Option<&mut FileBuffer> {
        self.current_buffer.as_mut()
    }

    /// 指定のバッファを現在のバッファとして読み込む
    pub fn set_current_buffer(&mut self, buffer: FileBuffer) {
        self.current_buffer = Some(buffer);
        if let Some(ref current) = self.current_buffer {
            self.editor = TextEditor::from_str(&current.content);
        } else {
            self.editor = TextEditor::new();
        }
        self.reset_command_context();
    }

    /// エディタ内容を同期
    pub fn sync_editor_content(&mut self, content: &str) {
        // 内部エディタを現在の内容で更新
        self.editor = TextEditor::from_str(content);

        // バッファがない場合は新規作成
        if self.current_buffer.is_none() {
            use crate::file::FileChangeTracker;

            // 新規バッファの場合、空の内容から開始してから現在の内容に更新
            let change_tracker = FileChangeTracker::new("");

            self.current_buffer = Some(FileBuffer {
                name: "untitled".to_string(),
                path: None,
                content: content.to_string(),
                change_tracker,
                file_info: None,
                read_only: false,
            });
        } else if let Some(ref mut buffer) = self.current_buffer {
            // バッファの内容を更新
            buffer.content = content.to_string();
            // 変更追跡は is_modified で確認できるので、特別な操作は不要
        }

        self.last_command = LastCommand::Other;
        self.last_yank_range = None;
    }

    fn record_kill(&mut self, text: String, join: KillJoin) {
        if text.is_empty() {
            return;
        }

        match join {
            KillJoin::Append => {
                if matches!(self.last_command, LastCommand::Kill) {
                    self.kill_ring.append_to_front(&text);
                } else {
                    self.kill_ring.push(text);
                }
            }
            KillJoin::Prepend => {
                if matches!(self.last_command, LastCommand::Kill) {
                    self.kill_ring.prepend_to_front(&text);
                } else {
                    self.kill_ring.push(text);
                }
            }
        }

        self.last_command = LastCommand::Kill;
        self.last_yank_range = None;
    }

    fn reset_command_context(&mut self) {
        self.last_command = LastCommand::Other;
        self.last_yank_range = None;
    }

    /// パスでファイルを開く（公開API）
    pub fn open_file(&mut self, path: String) -> CommandResult {
        self.execute_find_file_with_path(path)
    }

    /// コマンドを実行
    pub fn execute(&mut self, command: Command) -> CommandResult {
        match command {
            Command::ForwardChar => self.navigate(NavigationAction::MoveCharForward),
            Command::BackwardChar => self.navigate(NavigationAction::MoveCharBackward),
            Command::NextLine => self.navigate(NavigationAction::MoveLineDown),
            Command::PreviousLine => self.navigate(NavigationAction::MoveLineUp),
            Command::ForwardWord => self.navigate(NavigationAction::MoveWordForward),
            Command::BackwardWord => self.navigate(NavigationAction::MoveWordBackward),
            Command::InsertChar(ch) => {
                let res = self.editor.insert_char(ch);
                self.handle_edit(res)
            }
            Command::DeleteBackwardChar => {
                let res = self.editor.delete_backward();
                self.handle_delete(res)
            }
            Command::DeleteChar => {
                let res = self.editor.delete_forward();
                self.handle_delete(res)
            }
            Command::InsertNewline => {
                let res = self.editor.insert_newline();
                self.handle_edit(res)
            }
            Command::KillWordForward => {
                let res = self.editor.delete_word_forward();
                self.handle_kill(res, KillJoin::Append)
            }
            Command::KillWordBackward => {
                let res = self.editor.delete_word_backward();
                self.handle_kill(res, KillJoin::Prepend)
            }
            Command::KillLine => {
                let res = self.editor.kill_line_forward();
                self.handle_kill(res, KillJoin::Append)
            }
            Command::Yank => self.handle_yank(),
            Command::YankPop => self.handle_yank_pop(),
            Command::KeyboardQuit => self.handle_keyboard_quit(),
            Command::ScrollPageDown
            | Command::ScrollPageUp
            | Command::Recenter
            | Command::ScrollLeft
            | Command::ScrollRight
            | Command::SplitWindowBelow
            | Command::SplitWindowRight
            | Command::DeleteOtherWindows
            | Command::DeleteWindow
            | Command::OtherWindow
            | Command::SwitchToBuffer
            | Command::KillBuffer
            | Command::ListBuffers
            | Command::SetMark
            | Command::KillRegion
            | Command::CopyRegion
            | Command::ExchangePointAndMark
            | Command::MarkBuffer
            | Command::QueryReplace
            | Command::RegexQueryReplace => {
                CommandResult::error("このコマンドはアプリ側で処理します".to_string())
            }
            Command::FindFile => self.execute_find_file(),
            Command::SaveBuffer => self.execute_save_buffer(),
            Command::WriteFile => self.execute_write_file(),
            Command::SaveAllBuffers => self.execute_save_all_buffers(),
            Command::SaveBuffersKillTerminal => self.execute_quit(),
            Command::Quit => self.execute_quit(),
            Command::ExecuteCommand => self.execute_execute_command(),
            Command::EvalExpression => self.execute_eval_expression(),
            Command::MoveLineStart => self.navigate(NavigationAction::MoveLineStart),
            Command::MoveLineEnd => self.navigate(NavigationAction::MoveLineEnd),
            Command::MoveBufferStart => self.navigate(NavigationAction::MoveBufferStart),
            Command::MoveBufferEnd => self.navigate(NavigationAction::MoveBufferEnd),
            Command::Undo | Command::Redo => CommandResult::error("このコマンドはアプリ側で処理します".to_string()),
            Command::Unknown(cmd) => CommandResult::error(format!("不明なコマンド: {}", cmd)),
        }
    }

    fn navigate(&mut self, action: NavigationAction) -> CommandResult {
        match self.editor.navigate(action) {
            Ok(_) => {
                self.reset_command_context();
                CommandResult::success()
            }
            Err(err) => {
                self.reset_command_context();
                CommandResult::error(err.to_string())
            }
        }
    }

    fn handle_edit<T>(&mut self, result: crate::error::Result<T>) -> CommandResult {
        match result {
            Ok(_) => {
                self.reset_command_context();
                CommandResult::success()
            }
            Err(err) => {
                self.reset_command_context();
                CommandResult::error(err.to_string())
            }
        }
    }

    fn handle_delete(&mut self, result: crate::error::Result<char>) -> CommandResult {
        match result {
            Ok(_) => {
                self.reset_command_context();
                CommandResult::success()
            }
            Err(err) => {
                self.reset_command_context();
                CommandResult::error(err.to_string())
            }
        }
    }

    fn handle_kill(&mut self, result: crate::error::Result<String>, join: KillJoin) -> CommandResult {
        match result {
            Ok(text) => {
                if text.is_empty() {
                    self.reset_command_context();
                    return CommandResult::success();
                }

                self.record_kill(text, join);
                CommandResult::success()
            }
            Err(err) => {
                self.reset_command_context();
                CommandResult::error(err.to_string())
            }
        }
    }

    fn handle_yank(&mut self) -> CommandResult {
        let Some(text) = self.kill_ring.yank() else {
            self.reset_command_context();
            return CommandResult::error("キルリングが空です".to_string());
        };

        let start = self.editor.cursor().char_pos;
        let len = text.chars().count();

        match self.editor.insert_str(&text) {
            Ok(_) => {
                self.last_command = LastCommand::Yank;
                self.last_yank_range = Some((start, len));
                CommandResult::success()
            }
            Err(err) => {
                self.reset_command_context();
                CommandResult::error(err.to_string())
            }
        }
    }

    fn handle_yank_pop(&mut self) -> CommandResult {
        if !matches!(self.last_command, LastCommand::Yank) {
            self.reset_command_context();
            return CommandResult::error("直前のコマンドがヤンクではありません".to_string());
        }

        let Some((start, previous_len)) = self.last_yank_range else {
            self.reset_command_context();
            return CommandResult::error("ヤンク範囲を特定できません".to_string());
        };

        if let Err(err) = self.editor.move_cursor_to_char(start) {
            self.reset_command_context();
            return CommandResult::error(err.to_string());
        }

        if let Err(err) = self.editor.delete_range(start, start + previous_len) {
            self.reset_command_context();
            return CommandResult::error(err.to_string());
        }

        let Some(next_text) = self.kill_ring.rotate() else {
            self.reset_command_context();
            return CommandResult::error("キルリングが空です".to_string());
        };

        let new_len = next_text.chars().count();
        if let Err(err) = self.editor.insert_str(&next_text) {
            self.reset_command_context();
            return CommandResult::error(err.to_string());
        }

        self.last_command = LastCommand::Yank;
        self.last_yank_range = Some((start, new_len));
        CommandResult::success()
    }

    fn handle_keyboard_quit(&mut self) -> CommandResult {
        self.reset_command_context();
        CommandResult::success_with_message("操作をキャンセルしました".to_string())
    }

    fn execute_find_file(&mut self) -> CommandResult {
        self.reset_command_context();
        // TODO: ミニバッファでファイルパス入力を受け付ける実装が必要
        // 現在は簡易実装として仮のパスを使用
        self.execute_find_file_with_path("README.md".to_string())
    }

    fn execute_find_file_with_path(&mut self, path_input: String) -> CommandResult {
        self.reset_command_context();
        // パス展開
        let expanded_path = match expand_path(&path_input) {
            Ok(path) => path,
            Err(err) => return CommandResult::error(format!("パス展開エラー: {}", err)),
        };

        // ファイルを開く
        match self.file_manager.open_file(expanded_path.clone()) {
            Ok(buffer) => {
                self.set_current_buffer(buffer);

                CommandResult::success_with_message(
                    format!("ファイルを開きました: {}", expanded_path.display())
                )
            },
            Err(_err) => {
                // 新規ファイルの場合
                match self.file_manager.create_new_file_buffer(expanded_path.clone()) {
                    Ok(buffer) => {
                        self.set_current_buffer(buffer);

                        CommandResult::success_with_message(
                            format!("新規ファイルを作成しました: {}", expanded_path.display())
                        )
                    },
                    Err(create_err) => CommandResult::error(
                        format!("ファイル操作エラー: {}", create_err)
                    ),
                }
            }
        }
    }

    fn execute_save_buffer(&mut self) -> CommandResult {
        self.reset_command_context();
        if let Some(ref mut buffer) = self.current_buffer {
            // エディタの内容をバッファに同期
            buffer.content = self.editor.to_string();

            // 変更がない場合はスキップ
            if !buffer.is_modified() {
                return CommandResult::success_with_message("変更なし".to_string());
            }

            // 保存実行
            match self.file_manager.save_buffer(buffer) {
                Ok(_) => CommandResult::success_with_message(
                    format!("バッファを保存しました: {}",
                        buffer.path.as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "未名".to_string())
                    )
                ),
                Err(err) => CommandResult::error(format!("保存エラー: {}", err)),
            }
        } else {
            CommandResult::error("保存するバッファがありません".to_string())
        }
    }

    /// write-file コマンド実行（C-x C-w相当）
    fn execute_write_file(&mut self) -> CommandResult {
        CommandResult::error("write-file はミニバッファ経由で実行してください".to_string())
    }

    /// save-some-buffers コマンド実行（C-x s相当）
    fn execute_save_all_buffers(&mut self) -> CommandResult {
        // 現在は単一バッファのみ対応、将来的に複数バッファ対応時に拡張
        if let Some(ref buffer) = self.current_buffer {
            if buffer.path.is_some() {
                return self.execute_save_buffer();
            } else {
                return CommandResult::success_with_message("保存すべき変更がありません".to_string());
            }
        }
        CommandResult::error("保存するバッファがありません".to_string())
    }

    pub fn save_buffer_as(&mut self, path_input: String) -> CommandResult {
        self.reset_command_context();
        let expanded_path = match expand_path(&path_input) {
            Ok(path) => path,
            Err(err) => return CommandResult::error(format!("パス展開エラー: {}", err)),
        };

        if let Some(ref mut buffer) = self.current_buffer {
            buffer.content = self.editor.to_string();

        match self.file_manager.save_buffer_as(buffer, expanded_path.clone()) {
                Ok(_) => CommandResult::success_with_message(
                    format!("保存しました: {}", expanded_path.display())
                ),
                Err(err) => {
                    // より詳細なエラー情報を提供
                    eprintln!("保存エラーの詳細: {:?}", err);
                    eprintln!("保存先パス: {}", expanded_path.display());
                    CommandResult::error(format!("保存エラー: {} (パス: {})", err, expanded_path.display()))
                },
            }
        } else {
            CommandResult::error("保存するバッファがありません".to_string())
        }
    }

    fn execute_quit(&mut self) -> CommandResult {
        self.reset_command_context();
        CommandResult::quit()
    }

    fn execute_execute_command(&mut self) -> CommandResult {
        self.reset_command_context();
        CommandResult::success_with_message("M-x は現在未実装です".to_string())
    }

    fn execute_eval_expression(&mut self) -> CommandResult {
        self.reset_command_context();
        CommandResult::success_with_message("eval-expression はミニバッファで処理されます".to_string())
    }
}

impl Default for CommandProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// コマンドの実行コンテキスト
#[derive(Debug)]
pub struct CommandContext {
    /// 現在のバッファID
    pub buffer_id: Option<usize>,
    /// 引数
    pub args: Vec<String>,
    /// プレフィックス引数
    pub prefix_arg: Option<i32>,
}

impl CommandContext {
    /// 新しいコンテキストを作成
    pub fn new() -> Self {
        Self {
            buffer_id: None,
            args: Vec::new(),
            prefix_arg: None,
        }
    }

    /// 引数付きでコンテキストを作成
    pub fn with_args(args: Vec<String>) -> Self {
        Self {
            buffer_id: None,
            args,
            prefix_arg: None,
        }
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_from_string() {
        let cmd = Command::from_string("forward-char");
        match cmd {
            Command::ForwardChar => {},
            _ => panic!("Expected ForwardChar"),
        }

        let unknown = Command::from_string("unknown-command");
        match unknown {
            Command::Unknown(name) => assert_eq!(name, "unknown-command"),
            _ => panic!("Expected Unknown"),
        }

        assert!(matches!(Command::from_string("scroll-up"), Command::ScrollPageDown));
        assert!(matches!(Command::from_string("scroll-down"), Command::ScrollPageUp));
        assert!(matches!(Command::from_string("recenter-top-bottom"), Command::Recenter));
        assert!(matches!(Command::from_string("scroll-left"), Command::ScrollLeft));
        assert!(matches!(Command::from_string("scroll-right"), Command::ScrollRight));
        assert!(matches!(Command::from_string("set-mark-command"), Command::SetMark));
        assert!(matches!(Command::from_string("kill-region"), Command::KillRegion));
        assert!(matches!(Command::from_string("copy-region-as-kill"), Command::CopyRegion));
        assert!(matches!(Command::from_string("exchange-point-and-mark"), Command::ExchangePointAndMark));
        assert!(matches!(Command::from_string("mark-whole-buffer"), Command::MarkBuffer));
    }

    #[test]
    fn test_command_result() {
        let success = CommandResult::success();
        assert!(success.success);
        assert!(success.needs_refresh);
        assert!(!success.should_quit);

        let quit = CommandResult::quit();
        assert!(quit.success);
        assert!(quit.should_quit);
    }

    #[test]
    fn test_command_processor() {
        let mut processor = CommandProcessor::new();

        let result = processor.execute(Command::ForwardChar);
        assert!(result.success);

        let result = processor.execute(Command::InsertChar('a'));
        assert!(result.success);
        assert_eq!(processor.editor().to_string(), "a");
        assert_eq!(processor.editor().to_string(), "a");

        let result = processor.execute(Command::MoveLineStart);
        assert!(result.success);

        let result = processor.execute(Command::Quit);
        assert!(result.should_quit);

        let result = processor.execute(Command::Unknown("test".to_string()));
        assert!(!result.success);
    }

    #[test]
    fn test_kill_line_and_yank() {
        let mut processor = CommandProcessor::new();
        processor.sync_editor_content("hello\nworld");

        let result = processor.execute(Command::KillLine);
        assert!(result.success);
        assert_eq!(processor.editor().to_string(), "world");

        let result = processor.execute(Command::Yank);
        assert!(result.success);
        assert_eq!(processor.editor().to_string(), "hello\nworld");
    }

    #[test]
    fn test_yank_pop_rotates_entries() {
        let mut processor = CommandProcessor::new();
        processor.sync_editor_content("alpha beta gamma");

        processor.execute(Command::KillWordForward);
        processor.execute(Command::KillWordForward);

        processor.execute(Command::MoveLineStart);
        let yank_res = processor.execute(Command::Yank);
        assert!(yank_res.success);

        let pop_res = processor.execute(Command::YankPop);
        assert!(pop_res.success);
        assert!(processor.editor().to_string().contains("beta"));
    }
}
