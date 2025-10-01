//! コマンド処理システム
//!
//! M-x、find-file、save-bufferなどのコマンド実行機能

use crate::error::{AltreError, Result};
use std::collections::HashMap;
use std::path::PathBuf;

/// コマンドの実行結果
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// 成功
    Success(String),
    /// 失敗
    Error(String),
    /// ファイル操作が必要
    FileOperation(FileOperationType),
    /// バッファ操作が必要
    BufferOperation(BufferOperationType),
    /// システム操作が必要
    SystemOperation(SystemOperationType),
    /// 何もしない
    None,
}

/// ファイル操作の種類
#[derive(Debug, Clone)]
pub enum FileOperationType {
    /// ファイルを開く
    Open { path: PathBuf },
    /// ファイルを保存
    Save,
    /// 名前を付けて保存
    SaveAs { path: PathBuf },
    /// ファイルを閉じる
    Close,
    /// 新しいファイルを作成
    New,
}

/// バッファ操作の種類
#[derive(Debug, Clone)]
pub enum BufferOperationType {
    /// バッファを切り替え
    Switch { buffer_id: usize },
    /// バッファを削除
    Delete { buffer_id: usize },
    /// 全バッファをリスト
    List,
    /// バッファをリロード
    Reload,
}

/// システム操作の種類
#[derive(Debug, Clone)]
pub enum SystemOperationType {
    /// 終了
    Quit,
    /// 強制終了
    ForceQuit,
    /// 設定を開く
    OpenConfig,
    /// ヘルプを表示
    ShowHelp,
}

/// コマンドの定義
#[derive(Debug, Clone)]
pub struct CommandDefinition {
    /// コマンド名
    pub name: String,
    /// 説明
    pub description: String,
    /// 実行可能かどうかのチェック関数
    pub can_execute: fn(&CommandContext) -> bool,
    /// キーバインド（省略可能）
    pub keybinding: Option<String>,
    /// 引数の説明
    pub args_description: Option<String>,
}

/// コマンド実行のコンテキスト
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// 現在のファイルパス
    pub current_file: Option<PathBuf>,
    /// 現在のバッファID
    pub current_buffer_id: Option<usize>,
    /// バッファが変更されているか
    pub buffer_modified: bool,
    /// 利用可能なバッファ数
    pub buffer_count: usize,
    /// ファイルシステムのルートパス
    pub root_path: PathBuf,
}

impl Default for CommandContext {
    fn default() -> Self {
        Self {
            current_file: None,
            current_buffer_id: None,
            buffer_modified: false,
            buffer_count: 0,
            root_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

/// コマンド処理システム
pub struct CommandProcessor {
    /// 登録されたコマンド
    commands: HashMap<String, CommandDefinition>,
    /// コマンドエイリアス
    aliases: HashMap<String, String>,
    /// 実行履歴
    history: Vec<String>,
    /// 最大履歴サイズ
    max_history_size: usize,
}

impl CommandProcessor {
    /// 新しいコマンドプロセッサを作成
    pub fn new() -> Self {
        let mut processor = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
            history: Vec::new(),
            max_history_size: 100,
        };

        processor.register_builtin_commands();
        processor
    }

    /// 組み込みコマンドを登録
    fn register_builtin_commands(&mut self) {
        // ファイル操作コマンド
        self.register_command(CommandDefinition {
            name: "find-file".to_string(),
            description: "ファイルを開く".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x C-f".to_string()),
            args_description: Some("filename".to_string()),
        });

        self.register_command(CommandDefinition {
            name: "save-buffer".to_string(),
            description: "現在のバッファを保存".to_string(),
            can_execute: |ctx| ctx.current_file.is_some(),
            keybinding: Some("C-x C-s".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "write-file".to_string(),
            description: "別名でファイルを保存".to_string(),
            can_execute: |ctx| ctx.current_buffer_id.is_some(),
            keybinding: Some("C-x C-w".to_string()),
            args_description: Some("filename".to_string()),
        });

        self.register_command(CommandDefinition {
            name: "save-buffer-as".to_string(),
            description: "名前を付けてバッファを保存".to_string(),
            can_execute: |ctx| ctx.current_buffer_id.is_some(),
            keybinding: None,
            args_description: Some("filename".to_string()),
        });

        self.register_command(CommandDefinition {
            name: "save-some-buffers".to_string(),
            description: "すべてのバッファを保存".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x s".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "split-window-below".to_string(),
            description: "ウィンドウを上下に分割".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x 2".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "split-window-right".to_string(),
            description: "ウィンドウを左右に分割".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x 3".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "delete-other-windows".to_string(),
            description: "現在のウィンドウのみ表示".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x 1".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "delete-window".to_string(),
            description: "現在のウィンドウを閉じる".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x 0".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "other-window".to_string(),
            description: "次のウィンドウに移動".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x o".to_string()),
            args_description: None,
        });

        // バッファ操作コマンド
        self.register_command(CommandDefinition {
            name: "switch-to-buffer".to_string(),
            description: "バッファを切り替え".to_string(),
            can_execute: |ctx| ctx.buffer_count > 1,
            keybinding: Some("C-x b".to_string()),
            args_description: Some("buffer-name".to_string()),
        });

        self.register_command(CommandDefinition {
            name: "list-buffers".to_string(),
            description: "バッファ一覧を表示".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x C-b".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "kill-buffer".to_string(),
            description: "バッファを削除".to_string(),
            can_execute: |ctx| ctx.current_buffer_id.is_some(),
            keybinding: Some("C-x k".to_string()),
            args_description: Some("buffer-name".to_string()),
        });

        // システムコマンド
        self.register_command(CommandDefinition {
            name: "quit".to_string(),
            description: "エディタを終了".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x C-c".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "save-buffers-kill-terminal".to_string(),
            description: "全バッファを保存してエディタを終了".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-x C-c".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "eval-expression".to_string(),
            description: "alisp式を評価".to_string(),
            can_execute: |_| true,
            keybinding: Some("M-:".to_string()),
            args_description: Some("expression".to_string()),
        });

        // エディタコマンド
        self.register_command(CommandDefinition {
            name: "forward-char".to_string(),
            description: "カーソルを右に移動".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-f".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "backward-char".to_string(),
            description: "カーソルを左に移動".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-b".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "next-line".to_string(),
            description: "カーソルを下に移動".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-n".to_string()),
            args_description: None,
        });

        self.register_command(CommandDefinition {
            name: "previous-line".to_string(),
            description: "カーソルを上に移動".to_string(),
            can_execute: |_| true,
            keybinding: Some("C-p".to_string()),
            args_description: None,
        });

        // エイリアス設定
        self.register_alias("ff".to_string(), "find-file".to_string());
        self.register_alias("sb".to_string(), "save-buffer".to_string());
        self.register_alias("exit".to_string(), "quit".to_string());
        self.register_alias("q".to_string(), "quit".to_string());
    }

    /// コマンドを登録
    pub fn register_command(&mut self, command: CommandDefinition) {
        self.commands.insert(command.name.clone(), command);
    }

    /// エイリアスを登録
    pub fn register_alias(&mut self, alias: String, command: String) {
        self.aliases.insert(alias, command);
    }

    /// コマンドを実行
    pub fn execute_command(
        &mut self,
        command_line: &str,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let parts: Vec<&str> = command_line.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(CommandResult::None);
        }

        let command_name = parts[0];
        let args: Vec<&str> = parts[1..].to_vec();

        // エイリアスの解決
        let resolved_name = if let Some(alias_target) = self.aliases.get(command_name) {
            alias_target.clone()
        } else {
            command_name.to_string()
        };

        // コマンドの取得
        let command_def = self.commands.get(&resolved_name)
            .ok_or_else(|| AltreError::Application(format!("Unknown command: {}", resolved_name)))?;

        // 実行可能性チェック
        if !(command_def.can_execute)(context) {
            return Ok(CommandResult::Error(format!(
                "Command '{}' cannot be executed in current context",
                resolved_name
            )));
        }

        // 履歴に追加
        self.add_to_history(command_line.to_string());

        // コマンド実行
        self.execute_builtin_command(&resolved_name, &args, context)
    }

    /// 組み込みコマンドを実行
    fn execute_builtin_command(
        &self,
        command: &str,
        args: &[&str],
        context: &CommandContext,
    ) -> Result<CommandResult> {
        match command {
            "find-file" => {
                if args.is_empty() {
                    Ok(CommandResult::Error("No filename specified".to_string()))
                } else {
                    let path = PathBuf::from(args[0]);
                    Ok(CommandResult::FileOperation(FileOperationType::Open { path }))
                }
            }
            "save-buffer" => {
                if context.current_file.is_some() {
                    Ok(CommandResult::FileOperation(FileOperationType::Save))
                } else {
                    Ok(CommandResult::Error("No file associated with buffer".to_string()))
                }
            }
            "write-file" => {
                if args.is_empty() {
                    Ok(CommandResult::Error("No filename specified".to_string()))
                } else {
                    let path = PathBuf::from(args[0]);
                    Ok(CommandResult::FileOperation(FileOperationType::SaveAs { path }))
                }
            }
            "save-some-buffers" => {
                // 現在は単一バッファのみ対応
                if context.current_file.is_some() {
                    Ok(CommandResult::FileOperation(FileOperationType::Save))
                } else {
                    Ok(CommandResult::Success("No buffers need saving".to_string()))
                }
            }
            "save-buffer-as" => {
                if args.is_empty() {
                    Ok(CommandResult::Error("No filename specified".to_string()))
                } else {
                    let path = PathBuf::from(args[0]);
                    Ok(CommandResult::FileOperation(FileOperationType::SaveAs { path }))
                }
            }
            "switch-to-buffer" => {
                if args.is_empty() {
                    Ok(CommandResult::BufferOperation(BufferOperationType::List))
                } else {
                    // バッファ名からIDを解決（簡略化）
                    Ok(CommandResult::BufferOperation(BufferOperationType::Switch { buffer_id: 0 }))
                }
            }
            "list-buffers" => {
                Ok(CommandResult::BufferOperation(BufferOperationType::List))
            }
            "kill-buffer" => {
                if let Some(buffer_id) = context.current_buffer_id {
                    Ok(CommandResult::BufferOperation(BufferOperationType::Delete { buffer_id }))
                } else {
                    Ok(CommandResult::Error("No buffer to kill".to_string()))
                }
            }
            "quit" | "save-buffers-kill-terminal" => {
                Ok(CommandResult::SystemOperation(SystemOperationType::Quit))
            }
            "forward-char" | "backward-char" | "next-line" | "previous-line" => {
                // これらのコマンドはエディタ側で処理
                Ok(CommandResult::Success(format!("Executed: {}", command)))
            }
            _ => {
                Ok(CommandResult::Error(format!("Command not implemented: {}", command)))
            }
        }
    }

    /// 利用可能なコマンド一覧を取得
    pub fn list_commands(&self) -> Vec<&CommandDefinition> {
        self.commands.values().collect()
    }

    /// コマンド名で補完
    pub fn complete_command(&self, prefix: &str) -> Vec<String> {
        let mut matches = Vec::new();

        // コマンド名のマッチング
        for command_name in self.commands.keys() {
            if command_name.starts_with(prefix) {
                matches.push(command_name.clone());
            }
        }

        // エイリアスのマッチング
        for alias in self.aliases.keys() {
            if alias.starts_with(prefix) {
                matches.push(alias.clone());
            }
        }

        matches.sort();
        matches
    }

    /// コマンドの詳細情報を取得
    pub fn get_command_info(&self, command_name: &str) -> Option<&CommandDefinition> {
        // エイリアスの解決
        let resolved_name = if let Some(alias_target) = self.aliases.get(command_name) {
            alias_target.as_str()
        } else {
            command_name
        };

        self.commands.get(resolved_name)
    }

    /// 履歴に追加
    fn add_to_history(&mut self, command: String) {
        // 重複を避ける
        if let Some(last) = self.history.last() {
            if last == &command {
                return;
            }
        }

        self.history.push(command);

        // 履歴サイズの制限
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }
    }

    /// 実行履歴を取得
    pub fn get_history(&self) -> &[String] {
        &self.history
    }

    /// 履歴をクリア
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// コマンドが存在するかチェック
    pub fn command_exists(&self, command_name: &str) -> bool {
        self.commands.contains_key(command_name) || self.aliases.contains_key(command_name)
    }

    /// 実行可能なコマンドをフィルタ
    pub fn executable_commands(&self, context: &CommandContext) -> Vec<&CommandDefinition> {
        self.commands
            .values()
            .filter(|cmd| (cmd.can_execute)(context))
            .collect()
    }
}

impl Default for CommandProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_processor_creation() {
        let processor = CommandProcessor::new();
        assert!(processor.command_exists("find-file"));
        assert!(processor.command_exists("save-buffer"));
        assert!(processor.command_exists("quit"));
    }

    #[test]
    fn test_command_aliases() {
        let processor = CommandProcessor::new();
        assert!(processor.command_exists("ff")); // alias for find-file
        assert!(processor.command_exists("sb")); // alias for save-buffer
        assert!(processor.command_exists("q"));  // alias for quit
    }

    #[test]
    fn test_find_file_command() {
        let mut processor = CommandProcessor::new();
        let context = CommandContext::default();

        let result = processor.execute_command("find-file test.txt", &context).unwrap();
        match result {
            CommandResult::FileOperation(FileOperationType::Open { path }) => {
                assert_eq!(path, PathBuf::from("test.txt"));
            }
            _ => panic!("Expected FileOperation::Open"),
        }
    }

    #[test]
    fn test_save_buffer_command() {
        let mut processor = CommandProcessor::new();
        let mut context = CommandContext::default();
        context.current_file = Some(PathBuf::from("test.txt"));

        let result = processor.execute_command("save-buffer", &context).unwrap();
        assert!(matches!(result, CommandResult::FileOperation(FileOperationType::Save)));
    }

    #[test]
    fn test_quit_command() {
        let mut processor = CommandProcessor::new();
        let context = CommandContext::default();

        let result = processor.execute_command("quit", &context).unwrap();
        assert!(matches!(result, CommandResult::SystemOperation(SystemOperationType::Quit)));
    }

    #[test]
    fn test_command_completion() {
        let processor = CommandProcessor::new();

        let completions = processor.complete_command("find");
        assert!(completions.contains(&"find-file".to_string()));

        let completions = processor.complete_command("save");
        assert!(completions.len() >= 2); // save-buffer, save-buffer-as
    }

    #[test]
    fn test_command_history() {
        let mut processor = CommandProcessor::new();
        let context = CommandContext::default();

        processor.execute_command("find-file test1.txt", &context).unwrap();
        processor.execute_command("find-file test2.txt", &context).unwrap();

        let history = processor.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "find-file test1.txt");
        assert_eq!(history[1], "find-file test2.txt");
    }

    #[test]
    fn test_unknown_command() {
        let mut processor = CommandProcessor::new();
        let context = CommandContext::default();

        let result = processor.execute_command("unknown-command", &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_command_execution_context() {
        let processor = CommandProcessor::new();
        let mut context = CommandContext::default();

        // save-bufferは現在のファイルがない場合実行不可
        let save_cmd = processor.get_command_info("save-buffer").unwrap();
        assert!(!(save_cmd.can_execute)(&context));

        // ファイルを設定すると実行可能になる
        context.current_file = Some(PathBuf::from("test.txt"));
        assert!((save_cmd.can_execute)(&context));
    }

    #[test]
    fn test_executable_commands_filter() {
        let processor = CommandProcessor::new();
        let mut context = CommandContext::default();

        let executable_without_file = processor.executable_commands(&context);
        let find_file_available = executable_without_file
            .iter()
            .any(|cmd| cmd.name == "find-file");
        assert!(find_file_available);

        let save_buffer_available = executable_without_file
            .iter()
            .any(|cmd| cmd.name == "save-buffer");
        assert!(!save_buffer_available);

        // ファイルがある場合
        context.current_file = Some(PathBuf::from("test.txt"));
        let executable_with_file = processor.executable_commands(&context);
        let save_buffer_available_with_file = executable_with_file
            .iter()
            .any(|cmd| cmd.name == "save-buffer");
        assert!(save_buffer_available_with_file);
    }
}
