# キーバインド定義ファイル仕様

## 概要

本文書は、Altreテキストエディタのキーバインド設定ファイル（将来実装）の仕様を定義する。MVPでは組み込みキーバインドのみサポートするが、将来のカスタマイズ機能への拡張性を考慮した設計を行う。

## 設計目標

1. **可読性**: 人間が理解しやすい設定ファイル形式
2. **拡張性**: 将来の機能追加への対応
3. **エラーハンドリング**: 不正な設定の適切な検出と報告
4. **パフォーマンス**: 設定読み込みの高速化

## ファイル形式

### 基本構造（TOML形式）

```toml
# Altre キーバインド設定ファイル
# ~/.config/altre/keybindings.toml

[metadata]
version = "1.0"
description = "Autre custom keybindings"
author = "username"

[global]
# グローバルキーバインド（全モードで有効）

[global.single]
# 単一キーバインド
"C-n" = "move-cursor-down"
"C-p" = "move-cursor-up"
"C-f" = "move-cursor-right"
"C-b" = "move-cursor-left"
"M-x" = "execute-command"

# 矢印キー
"Up" = "move-cursor-up"
"Down" = "move-cursor-down"
"Left" = "move-cursor-left"
"Right" = "move-cursor-right"

# 編集操作
"Backspace" = "delete-backward-char"
"Delete" = "delete-forward-char"
"Enter" = "insert-newline"
"C-j" = "newline-and-indent"
"C-o" = "open-line"

[global.sequence]
# 連続キーバインド
"C-x C-f" = "file-open"
"C-x C-s" = "file-save"
"C-x C-c" = "quit"

[modes]
# モード固有のキーバインド（将来実装）

[modes.edit]
# 編集モード

[modes.command]
# コマンドモード

[aliases]
# コマンドエイリアス
"forward-char" = "move-cursor-right"
"backward-char" = "move-cursor-left"
"next-line" = "move-cursor-down"
"previous-line" = "move-cursor-up"
"find-file" = "file-open"
"save-buffer" = "file-save"
"save-buffers-kill-terminal" = "quit"

[preferences]
# キーバインド動作設定
partial_match_timeout_ms = 1000
case_sensitive = false
enable_prefix_keys = true
silent_unbound = true
```

### キー表記法

#### 修飾キー記法

| 記法 | 意味 | 例 |
|------|------|-----|
| `C-` | Ctrl | `C-x` = Ctrl+x |
| `M-` | Alt/Meta | `M-x` = Alt+x |
| `S-` | Shift | `S-F1` = Shift+F1 |
| `C-M-` | Ctrl+Alt | `C-M-x` = Ctrl+Alt+x |

#### 特殊キー記法

| 記法 | 意味 |
|------|------|
| `Enter` | エンターキー |
| `Backspace` | バックスペースキー |
| `Delete` | デリートキー |
| `Tab` | タブキー |
| `Esc` | エスケープキー |
| `Space` | スペースキー |
| `Up`/`Down`/`Left`/`Right` | 矢印キー |
| `F1`-`F12` | ファンクションキー |
| `Home`/`End` | ホーム/エンドキー |
| `PageUp`/`PageDown` | ページアップ/ダウンキー |

#### 連続キー記法

```toml
# スペース区切りで連続キーを表現
"C-x C-f" = "file-open"
"C-x C-s" = "file-save"
"C-c C-c" = "compile"

# より複雑な例
"C-x r t" = "string-rectangle"
"C-u C-x C-f" = "file-open-with-encoding"
```

### アクション定義

#### 組み込みアクション

```toml
[actions.builtin]
# カーソル移動
"move-cursor-up" = { type = "cursor", direction = "up" }
"move-cursor-down" = { type = "cursor", direction = "down" }
"move-cursor-left" = { type = "cursor", direction = "left" }
"move-cursor-right" = { type = "cursor", direction = "right" }

# 編集操作
"delete-backward-char" = { type = "delete", direction = "backward" }
"delete-forward-char" = { type = "delete", direction = "forward" }
"insert-newline" = { type = "insert", content = "\n" }
"newline-and-indent" = { type = "editor", action = "newline-indent" }
"open-line" = { type = "editor", action = "open-line" }

# ファイル操作
"file-open" = { type = "file", action = "open" }
"file-save" = { type = "file", action = "save" }
"quit" = { type = "app", action = "quit" }

# コマンド実行
"execute-command" = { type = "command", action = "execute" }
```

#### カスタムアクション（将来実装）

```toml
[actions.custom]
# Lua スクリプト実行
"my-custom-action" = { type = "lua", script = "custom_actions/my_action.lua" }

# シェルコマンド実行
"run-make" = { type = "shell", command = "make", args = ["build"] }

# 複合アクション
"save-and-compile" = {
    type = "sequence",
    actions = ["file-save", "run-make"]
}
```

## 設定ファイル処理

### パース処理

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct KeybindingConfig {
    pub metadata: ConfigMetadata,
    pub global: GlobalKeybindings,
    pub modes: Option<HashMap<String, ModeKeybindings>>,
    pub aliases: Option<HashMap<String, String>>,
    pub preferences: Option<KeybindingPreferences>,
    pub actions: Option<ActionDefinitions>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigMetadata {
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalKeybindings {
    pub single: HashMap<String, String>,
    pub sequence: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModeKeybindings {
    pub single: Option<HashMap<String, String>>,
    pub sequence: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeybindingPreferences {
    pub partial_match_timeout_ms: Option<u64>,
    pub case_sensitive: Option<bool>,
    pub enable_prefix_keys: Option<bool>,
    pub silent_unbound: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionDefinitions {
    pub builtin: Option<HashMap<String, ActionSpec>>,
    pub custom: Option<HashMap<String, CustomActionSpec>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ActionSpec {
    #[serde(rename = "cursor")]
    Cursor { direction: String },

    #[serde(rename = "delete")]
    Delete { direction: String },

    #[serde(rename = "insert")]
    Insert { content: String },

    #[serde(rename = "file")]
    File { action: String },

    #[serde(rename = "app")]
    App { action: String },

    #[serde(rename = "command")]
    Command { action: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CustomActionSpec {
    #[serde(rename = "lua")]
    Lua { script: String },

    #[serde(rename = "shell")]
    Shell { command: String, args: Option<Vec<String>> },

    #[serde(rename = "sequence")]
    Sequence { actions: Vec<String> },
}
```

### 設定ファイル読み込み

```rust
impl KeybindingConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(e))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e))?;

        config.validate()?;
        Ok(config)
    }

    pub fn load_default() -> Self {
        // MVPデフォルト設定
        Self {
            metadata: ConfigMetadata {
                version: "1.0".to_string(),
                description: Some("Default MVP keybindings".to_string()),
                author: None,
            },
            global: GlobalKeybindings {
                single: Self::default_single_bindings(),
                sequence: Self::default_sequence_bindings(),
            },
            modes: None,
            aliases: Some(Self::default_aliases()),
            preferences: Some(KeybindingPreferences {
                partial_match_timeout_ms: Some(1000),
                case_sensitive: Some(false),
                enable_prefix_keys: Some(true),
                silent_unbound: Some(true),
            }),
            actions: None,
        }
    }

    fn default_single_bindings() -> HashMap<String, String> {
        let mut bindings = HashMap::new();

        // 移動系
        bindings.insert("C-n".to_string(), "move-cursor-down".to_string());
        bindings.insert("C-p".to_string(), "move-cursor-up".to_string());
        bindings.insert("C-f".to_string(), "move-cursor-right".to_string());
        bindings.insert("C-b".to_string(), "move-cursor-left".to_string());

        // 矢印キー
        bindings.insert("Up".to_string(), "move-cursor-up".to_string());
        bindings.insert("Down".to_string(), "move-cursor-down".to_string());
        bindings.insert("Left".to_string(), "move-cursor-left".to_string());
        bindings.insert("Right".to_string(), "move-cursor-right".to_string());

        // 編集系
        bindings.insert("Backspace".to_string(), "delete-backward-char".to_string());
        bindings.insert("Delete".to_string(), "delete-forward-char".to_string());
        bindings.insert("Enter".to_string(), "insert-newline".to_string());
        bindings.insert("C-j".to_string(), "newline-and-indent".to_string());
        bindings.insert("C-o".to_string(), "open-line".to_string());

        // コマンド実行
        bindings.insert("M-x".to_string(), "execute-command".to_string());

        bindings
    }

    fn default_sequence_bindings() -> HashMap<String, String> {
        let mut bindings = HashMap::new();

        bindings.insert("C-x C-f".to_string(), "file-open".to_string());
        bindings.insert("C-x C-s".to_string(), "file-save".to_string());
        bindings.insert("C-x C-c".to_string(), "quit".to_string());

        bindings
    }

    fn default_aliases() -> HashMap<String, String> {
        let mut aliases = HashMap::new();

        aliases.insert("forward-char".to_string(), "move-cursor-right".to_string());
        aliases.insert("backward-char".to_string(), "move-cursor-left".to_string());
        aliases.insert("next-line".to_string(), "move-cursor-down".to_string());
        aliases.insert("previous-line".to_string(), "move-cursor-up".to_string());
        aliases.insert("find-file".to_string(), "file-open".to_string());
        aliases.insert("save-buffer".to_string(), "file-save".to_string());
        aliases.insert("save-buffers-kill-terminal".to_string(), "quit".to_string());

        aliases
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // バージョンチェック
        if !self.is_version_supported(&self.metadata.version) {
            return Err(ConfigError::UnsupportedVersion(self.metadata.version.clone()));
        }

        // キーバインド形式の検証
        self.validate_keybindings()?;

        // アクションの存在確認
        self.validate_actions()?;

        Ok(())
    }

    fn validate_keybindings(&self) -> Result<(), ConfigError> {
        // 単一キーの検証
        for (key_str, action) in &self.global.single {
            KeySequence::parse(key_str)
                .map_err(|e| ConfigError::InvalidKeyBinding(key_str.clone(), e))?;

            if action.is_empty() {
                return Err(ConfigError::EmptyAction(key_str.clone()));
            }
        }

        // 連続キーの検証
        for (key_str, action) in &self.global.sequence {
            let sequence = KeySequence::parse(key_str)
                .map_err(|e| ConfigError::InvalidKeyBinding(key_str.clone(), e))?;

            if sequence.keys.len() < 2 {
                return Err(ConfigError::InvalidSequenceLength(key_str.clone()));
            }

            if action.is_empty() {
                return Err(ConfigError::EmptyAction(key_str.clone()));
            }
        }

        Ok(())
    }

    fn validate_actions(&self) -> Result<(), ConfigError> {
        // アクションの存在確認は実装時に詳細化
        Ok(())
    }

    fn is_version_supported(&self, version: &str) -> bool {
        // セマンティックバージョニングのチェック
        version.starts_with("1.")
    }
}
```

### エラーハンドリング

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Unsupported config version: {0}")]
    UnsupportedVersion(String),

    #[error("Invalid key binding '{0}': {1}")]
    InvalidKeyBinding(String, KeyParseError),

    #[error("Empty action for key binding '{0}'")]
    EmptyAction(String),

    #[error("Invalid sequence length for '{0}' (must be 2+ keys)")]
    InvalidSequenceLength(String),

    #[error("Unknown action '{0}'")]
    UnknownAction(String),

    #[error("Conflicting key binding: '{0}'")]
    ConflictingBinding(String),
}
```

## 設定ファイルの場所

### デフォルト検索パス

1. `$XDG_CONFIG_HOME/autre/keybindings.toml`
2. `$HOME/.config/autre/keybindings.toml`
3. `$HOME/.autre/keybindings.toml`
4. `./keybindings.toml` (現在のディレクトリ)

### 設定ファイル検索ロジック

```rust
impl KeybindingConfig {
    pub fn find_config_file() -> Option<PathBuf> {
        let search_paths = [
            Self::xdg_config_path(),
            Self::home_config_path(),
            Self::home_dot_path(),
            Self::current_dir_path(),
        ];

        for path in search_paths.iter().flatten() {
            if path.exists() {
                return Some(path.clone());
            }
        }

        None
    }

    fn xdg_config_path() -> Option<PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|dir| PathBuf::from(dir).join("autre/keybindings.toml"))
    }

    fn home_config_path() -> Option<PathBuf> {
        dirs::home_dir()
            .map(|dir| dir.join(".config/autre/keybindings.toml"))
    }

    fn home_dot_path() -> Option<PathBuf> {
        dirs::home_dir()
            .map(|dir| dir.join(".autre/keybindings.toml"))
    }

    fn current_dir_path() -> Option<PathBuf> {
        std::env::current_dir()
            .ok()
            .map(|dir| dir.join("keybindings.toml"))
    }
}
```

## マイグレーション戦略

### バージョン間の互換性

```rust
impl KeybindingConfig {
    pub fn migrate_from_version(content: &str, from_version: &str) -> Result<Self, ConfigError> {
        match from_version {
            "0.9" => Self::migrate_from_0_9(content),
            "1.0" => toml::from_str(content).map_err(ConfigError::Parse),
            _ => Err(ConfigError::UnsupportedVersion(from_version.to_string())),
        }
    }

    fn migrate_from_0_9(content: &str) -> Result<Self, ConfigError> {
        // 旧形式からの変換ロジック
        // 実装時に詳細化
        todo!("Implement migration from 0.9")
    }
}
```

## 統合とテスト

### ModernKeyMapとの統合

```rust
impl ModernKeyMap {
    pub fn from_config(config: &KeybindingConfig) -> Result<Self, ConfigError> {
        let mut keymap = Self::new();

        // 設定からキーバインドを読み込み
        keymap.load_bindings(&config.global)?;

        // エイリアスの適用
        if let Some(aliases) = &config.aliases {
            keymap.apply_aliases(aliases)?;
        }

        // 設定の適用
        if let Some(prefs) = &config.preferences {
            keymap.apply_preferences(prefs);
        }

        Ok(keymap)
    }

    fn load_bindings(&mut self, global: &GlobalKeybindings) -> Result<(), ConfigError> {
        // 単一キーバインドの読み込み
        for (key_str, action_str) in &global.single {
            let key_seq = KeySequence::parse(key_str)?;
            if key_seq.keys.len() == 1 {
                let action = self.parse_action(action_str)?;
                self.single_key_bindings.insert(key_seq.keys[0].clone(), action);
            }
        }

        // 連続キーバインドの読み込み
        for (key_str, action_str) in &global.sequence {
            let key_seq = KeySequence::parse(key_str)?;
            if key_seq.keys.len() == 2 && key_seq.keys[0].is_ctrl_x() {
                let action = self.parse_action(action_str)?;
                self.cx_prefix_bindings.insert(key_seq.keys[1].clone(), action);
            }
        }

        Ok(())
    }

    fn parse_action(&self, action_str: &str) -> Result<Action, ConfigError> {
        match action_str {
            "move-cursor-up" => Ok(Action::MoveCursor(Direction::Up)),
            "move-cursor-down" => Ok(Action::MoveCursor(Direction::Down)),
            "move-cursor-left" => Ok(Action::MoveCursor(Direction::Left)),
            "move-cursor-right" => Ok(Action::MoveCursor(Direction::Right)),
            "delete-backward-char" => Ok(Action::DeleteChar(DeleteDirection::Backward)),
            "delete-forward-char" => Ok(Action::DeleteChar(DeleteDirection::Forward)),
            "insert-newline" => Ok(Action::InsertNewline),
            "newline-and-indent" => Ok(Action::NewlineAndIndent),
            "open-line" => Ok(Action::OpenLine),
            "file-open" => Ok(Action::FileOpen),
            "file-save" => Ok(Action::FileSave),
            "quit" => Ok(Action::Quit),
            "execute-command" => Ok(Action::ExecuteCommand),
            _ => Err(ConfigError::UnknownAction(action_str.to_string())),
        }
    }
}
```

### 設定ファイルのテスト

```rust
#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml_content = r#"
[metadata]
version = "1.0"
description = "Test config"

[global.single]
"C-n" = "move-cursor-down"
"C-p" = "move-cursor-up"

[global.sequence]
"C-x C-f" = "file-open"

[preferences]
partial_match_timeout_ms = 500
silent_unbound = true
"#;

        let config: KeybindingConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.metadata.version, "1.0");
        assert_eq!(config.global.single.len(), 2);
        assert_eq!(config.global.sequence.len(), 1);
    }

    #[test]
    fn test_invalid_key_binding() {
        let toml_content = r#"
[metadata]
version = "1.0"

[global.single]
"INVALID-KEY" = "move-cursor-down"
"#;

        let config: KeybindingConfig = toml::from_str(toml_content).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_keymap_from_config() {
        let config = KeybindingConfig::load_default();
        let keymap = ModernKeyMap::from_config(&config).unwrap();

        let result = keymap.process_key(Key::ctrl_n());
        assert_eq!(result, KeyProcessResult::Action(Action::MoveCursor(Direction::Down)));
    }
}
```

## 将来の拡張

### プラグインシステム連携

```toml
[plugins]
# プラグインからのキーバインド
enabled = ["git-integration", "lsp-client"]

[plugins.git-integration]
"C-x g s" = "git-status"
"C-x g c" = "git-commit"

[plugins.lsp-client]
"M-." = "lsp-goto-definition"
"M-?" = "lsp-find-references"
```

### 動的キーバインド

```toml
[dynamic]
# 条件付きキーバインド
[dynamic.file-type]
"*.rs" = { "C-c C-c" = "rust-compile" }
"*.py" = { "C-c C-c" = "python-run" }

[dynamic.mode]
insert = { "Tab" = "auto-complete" }
command = { "Tab" = "command-complete" }
```

## 制限事項

### MVPでの制約
- 設定ファイル機能は未実装（組み込みキーバインドのみ）
- モード固有キーバインドは未サポート
- カスタムアクションは未サポート
- プラグイン連携は未サポート

### 既知の制限
- 現時点では英語キーボードレイアウトのみ想定
- 一部の特殊キーはプラットフォーム依存
- IME使用時の制限

これらの制限は将来バージョンで段階的に解決予定。
