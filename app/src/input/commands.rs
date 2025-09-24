//! コマンドシステム
//!
//! エディタコマンドの定義と実行

use crate::buffer::{EditOperations, NavigationAction, TextEditor};
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

/// コマンドの種類
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // カーソル移動
    ForwardChar,
    BackwardChar,
    NextLine,
    PreviousLine,

    // 編集操作
    InsertChar(char),
    DeleteBackwardChar,
    DeleteChar,
    InsertNewline,

    // ファイル操作
    FindFile,
    SaveBuffer,

    // アプリケーション制御
    SaveBuffersKillTerminal,
    Quit,
    ExecuteCommand,
    MoveLineStart,
    MoveLineEnd,
    MoveBufferStart,
    MoveBufferEnd,
    EvalExpression,

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
            "delete-backward-char" => Command::DeleteBackwardChar,
            "delete-char" => Command::DeleteChar,
            "newline" => Command::InsertNewline,
            "find-file" => Command::FindFile,
            "save-buffer" => Command::SaveBuffer,
            "save-buffers-kill-terminal" => Command::SaveBuffersKillTerminal,
            "quit" => Command::Quit,
            "execute-command" => Command::ExecuteCommand,
            "eval-expression" => Command::EvalExpression,
            "move-beginning-of-line" => Command::MoveLineStart,
            "move-end-of-line" => Command::MoveLineEnd,
            "beginning-of-buffer" => Command::MoveBufferStart,
            "end-of-buffer" => Command::MoveBufferEnd,
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
            Command::InsertChar(_) => "文字を挿入",
            Command::DeleteBackwardChar => "前の文字を削除",
            Command::DeleteChar => "カーソル位置の文字を削除",
            Command::InsertNewline => "改行を挿入",
            Command::FindFile => "ファイルを開く",
            Command::SaveBuffer => "バッファを保存",
            Command::SaveBuffersKillTerminal => "保存して終了",
            Command::Quit => "終了",
            Command::ExecuteCommand => "コマンドを実行",
            Command::EvalExpression => "式を評価",
            Command::MoveLineStart => "行頭に移動",
            Command::MoveLineEnd => "行末に移動",
            Command::MoveBufferStart => "バッファ先頭に移動",
            Command::MoveBufferEnd => "バッファ末尾に移動",
            Command::Unknown(_) => "不明なコマンド",
        }
    }
}

/// コマンド処理器
pub struct CommandProcessor {
    editor: TextEditor,
    file_manager: FileOperationManager,
    current_buffer: Option<FileBuffer>,
}

impl CommandProcessor {
    /// 新しいコマンド処理器を作成
    pub fn new() -> Self {
        Self {
            editor: TextEditor::new(),
            file_manager: FileOperationManager::new(),
            current_buffer: None,
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
            Command::FindFile => self.execute_find_file(),
            Command::SaveBuffer => self.execute_save_buffer(),
            Command::SaveBuffersKillTerminal => self.execute_quit(),
            Command::Quit => self.execute_quit(),
            Command::ExecuteCommand => self.execute_execute_command(),
            Command::EvalExpression => self.execute_eval_expression(),
            Command::MoveLineStart => self.navigate(NavigationAction::MoveLineStart),
            Command::MoveLineEnd => self.navigate(NavigationAction::MoveLineEnd),
            Command::MoveBufferStart => self.navigate(NavigationAction::MoveBufferStart),
            Command::MoveBufferEnd => self.navigate(NavigationAction::MoveBufferEnd),
            Command::Unknown(cmd) => CommandResult::error(format!("不明なコマンド: {}", cmd)),
        }
    }

    fn navigate(&mut self, action: NavigationAction) -> CommandResult {
        match self.editor.navigate(action) {
            Ok(_) => CommandResult::success(),
            Err(err) => CommandResult::error(err.to_string()),
        }
    }

    fn handle_edit<T>(&mut self, result: crate::error::Result<T>) -> CommandResult {
        match result {
            Ok(_) => CommandResult::success(),
            Err(err) => CommandResult::error(err.to_string()),
        }
    }

    fn handle_delete(&mut self, result: crate::error::Result<char>) -> CommandResult {
        match result {
            Ok(_) => CommandResult::success(),
            Err(err) => CommandResult::error(err.to_string()),
        }
    }

    fn execute_find_file(&mut self) -> CommandResult {
        // TODO: ミニバッファでファイルパス入力を受け付ける実装が必要
        // 現在は簡易実装として仮のパスを使用
        self.execute_find_file_with_path("README.md".to_string())
    }

    fn execute_find_file_with_path(&mut self, path_input: String) -> CommandResult {
        // パス展開
        let expanded_path = match expand_path(&path_input) {
            Ok(path) => path,
            Err(err) => return CommandResult::error(format!("パス展開エラー: {}", err)),
        };

        // ファイルを開く
        match self.file_manager.open_file(expanded_path.clone()) {
            Ok(buffer) => {
                // エディタにファイル内容を設定
                self.editor = TextEditor::from_str(&buffer.content);

                // バッファを保存
                self.current_buffer = Some(buffer);

                CommandResult::success_with_message(
                    format!("ファイルを開きました: {}", expanded_path.display())
                )
            },
            Err(_err) => {
                // 新規ファイルの場合
                match self.file_manager.create_new_file_buffer(expanded_path.clone()) {
                    Ok(buffer) => {
                        self.editor = TextEditor::from_str(&buffer.content);
                        self.current_buffer = Some(buffer);

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

    pub fn save_buffer_as(&mut self, path_input: String) -> CommandResult {
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
                Err(err) => CommandResult::error(format!("保存エラー: {}", err)),
            }
        } else {
            CommandResult::error("保存するバッファがありません".to_string())
        }
    }

    fn execute_quit(&mut self) -> CommandResult {
        CommandResult::quit()
    }

    fn execute_execute_command(&mut self) -> CommandResult {
        CommandResult::success_with_message("M-x は現在未実装です".to_string())
    }

    fn execute_eval_expression(&mut self) -> CommandResult {
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
}
