# ミニバッファ設計仕様書

## 概要

本文書は、Altreテキストエディタのミニバッファシステムの設計仕様を定義する。Emacs風のコマンド入力インターフェースを提供し、ファイル操作、コマンド実行、エラー表示等の機能を統合する。

## 設計目標

1. **直感的操作**: 現代的なエディタ風の使いやすいインターフェース
2. **高性能**: 応答性の高い補完とキー入力処理
3. **拡張性**: 将来のコマンド機能拡張への対応
4. **統合性**: キーバインドシステムとの緊密な連携

## UI設計

### レイアウト配置

QA.mdの回答に基づき、画面上部に配置する現代的なエディタ風デザインを採用。

```
┌─────────────────────────────────────────────────────────────┐
│ [Minibuffer]  Find file: ~/project/src/main.rs             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Completion candidates (if active):                      │ │
│ │ • ~/project/src/main.rs                                 │ │
│ │ • ~/project/src/main.rs.bak                             │ │
│ │ • ~/project/src/main_test.rs                            │ │
│ └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│                Main Editor Area                             │
│                                                             │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ Status Line: ./main.rs | Line 1, Col 1 | Modified         │
└─────────────────────────────────────────────────────────────┘
```

### 視覚的要素

```rust
/// ミニバッファの外観設定
#[derive(Debug, Clone)]
pub struct MinibufferStyle {
    /// プロンプト部分のスタイル
    pub prompt_style: Style,
    /// 入力テキストのスタイル
    pub input_style: Style,
    /// 補完候補のスタイル
    pub completion_style: Style,
    /// 選択された候補のスタイル
    pub selected_completion_style: Style,
    /// エラーメッセージのスタイル
    pub error_style: Style,
    /// ボーダーのスタイル
    pub border_style: Style,
}

impl Default for MinibufferStyle {
    fn default() -> Self {
        Self {
            prompt_style: Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            input_style: Style::default().fg(Color::White),
            completion_style: Style::default().fg(Color::Gray),
            selected_completion_style: Style::default().bg(Color::Blue).fg(Color::White),
            error_style: Style::default().fg(Color::Red),
            border_style: Style::default().fg(Color::DarkGray),
        }
    }
}
```

## アーキテクチャ設計

### 状態管理

```rust
/// ミニバッファの動作モード
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferMode {
    /// 非アクティブ状態
    Inactive,
    /// ファイルパス入力
    FindFile,
    /// コマンド実行入力
    ExecuteCommand,
    /// 保存確認
    SaveConfirmation,
    /// エラーメッセージ表示
    ErrorDisplay { message: String, expires_at: Instant },
    /// 情報メッセージ表示
    InfoDisplay { message: String, expires_at: Instant },
}

/// ミニバッファの状態
#[derive(Debug, Clone)]
pub struct MinibufferState {
    /// 現在のモード
    pub mode: MinibufferMode,
    /// 入力中のテキスト
    pub input: String,
    /// カーソル位置（バイト単位）
    pub cursor_pos: usize,
    /// 現在のプロンプト
    pub prompt: String,
    /// 補完候補
    pub completions: Vec<String>,
    /// 選択中の補完候補インデックス
    pub selected_completion: Option<usize>,
    /// 履歴（セッション内のみ）
    pub history: Vec<String>,
    /// 履歴ナビゲーション位置
    pub history_index: Option<usize>,
}
```

### 入力処理システム

```rust
/// ミニバッファの入力イベント
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferEvent {
    /// 文字入力
    Input(char),
    /// Backspace
    Backspace,
    /// Delete
    Delete,
    /// カーソル移動
    MoveCursor(CursorDirection),
    /// Enter（確定）
    Submit,
    /// Tab（補完）
    Complete,
    /// キャンセル（C-g）
    Cancel,
    /// 履歴ナビゲーション
    HistoryPrevious,
    HistoryNext,
    /// 補完候補選択
    SelectCompletion(usize),
    /// 補完候補ナビゲーション
    CompletionNext,
    CompletionPrevious,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorDirection {
    Left,
    Right,
    Home,
    End,
}

/// 入力処理の結果
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferResult {
    /// 処理継続
    Continue,
    /// コマンド実行
    Execute(String),
    /// キャンセル
    Cancel,
    /// 無効な操作
    Invalid,
}
```

## コア機能実装

### メインコントローラー

```rust
/// ミニバッファコントローラー
#[derive(Debug)]
pub struct Minibuffer {
    /// 現在の状態
    state: MinibufferState,
    /// 外観設定
    style: MinibufferStyle,
    /// 補完エンジン
    completion_engine: Box<dyn CompletionEngine>,
    /// コマンド実行者
    command_executor: Box<dyn CommandExecutor>,
}

impl Minibuffer {
    /// 新しいミニバッファを作成
    pub fn new() -> Self {
        Self {
            state: MinibufferState::default(),
            style: MinibufferStyle::default(),
            completion_engine: Box::new(FileCompletionEngine::new()),
            command_executor: Box::new(DefaultCommandExecutor::new()),
        }
    }

    /// ファイル検索を開始
    pub fn start_find_file(&mut self, initial_path: Option<&str>) {
        self.state.mode = MinibufferMode::FindFile;
        self.state.prompt = "Find file: ".to_string();
        self.state.input = initial_path.unwrap_or("").to_string();
        self.state.cursor_pos = self.state.input.len();
        self.update_completions();
    }

    /// コマンド実行を開始
    pub fn start_execute_command(&mut self) {
        self.state.mode = MinibufferMode::ExecuteCommand;
        self.state.prompt = "M-x ".to_string();
        self.state.input.clear();
        self.state.cursor_pos = 0;
        self.update_completions();
    }

    /// エラーメッセージを表示
    pub fn show_error(&mut self, message: String) {
        let expires_at = Instant::now() + Duration::from_secs(5); // QA.mdの回答
        self.state.mode = MinibufferMode::ErrorDisplay { message, expires_at };
    }

    /// 情報メッセージを表示
    pub fn show_info(&mut self, message: String) {
        let expires_at = Instant::now() + Duration::from_secs(3);
        self.state.mode = MinibufferMode::InfoDisplay { message, expires_at };
    }

    /// キー入力を処理
    pub fn handle_key(&mut self, key: Key) -> MinibufferResult {
        // メッセージ表示中の自動消去チェック
        self.check_message_expiry();

        match self.state.mode {
            MinibufferMode::Inactive => MinibufferResult::Continue,
            MinibufferMode::ErrorDisplay { .. } | MinibufferMode::InfoDisplay { .. } => {
                // メッセージ表示中は任意のキーで消去
                self.deactivate();
                MinibufferResult::Continue
            }
            _ => self.handle_input_key(key),
        }
    }

    fn handle_input_key(&mut self, key: Key) -> MinibufferResult {
        let event = self.key_to_event(key);

        match event {
            MinibufferEvent::Input(ch) => {
                self.insert_char(ch);
                self.update_completions();
                MinibufferResult::Continue
            }
            MinibufferEvent::Backspace => {
                self.delete_backward();
                self.update_completions();
                MinibufferResult::Continue
            }
            MinibufferEvent::Delete => {
                self.delete_forward();
                self.update_completions();
                MinibufferResult::Continue
            }
            MinibufferEvent::Submit => {
                self.submit()
            }
            MinibufferEvent::Cancel => {
                self.cancel()
            }
            MinibufferEvent::Complete => {
                self.handle_completion();
                MinibufferResult::Continue
            }
            MinibufferEvent::MoveCursor(direction) => {
                self.move_cursor(direction);
                MinibufferResult::Continue
            }
            MinibufferEvent::CompletionNext => {
                self.select_next_completion();
                MinibufferResult::Continue
            }
            MinibufferEvent::CompletionPrevious => {
                self.select_previous_completion();
                MinibufferResult::Continue
            }
            _ => MinibufferResult::Continue,
        }
    }

    fn key_to_event(&self, key: Key) -> MinibufferEvent {
        match key.code {
            KeyCode::Char(ch) if !key.modifiers.ctrl && !key.modifiers.alt => {
                MinibufferEvent::Input(ch)
            }
            KeyCode::Backspace => MinibufferEvent::Backspace,
            KeyCode::Delete => MinibufferEvent::Delete,
            KeyCode::Enter => MinibufferEvent::Submit,
            KeyCode::Tab => MinibufferEvent::Complete,
            KeyCode::Char('g') if key.modifiers.ctrl => MinibufferEvent::Cancel,
            KeyCode::Left => MinibufferEvent::MoveCursor(CursorDirection::Left),
            KeyCode::Right => MinibufferEvent::MoveCursor(CursorDirection::Right),
            KeyCode::Char('a') if key.modifiers.ctrl => MinibufferEvent::MoveCursor(CursorDirection::Home),
            KeyCode::Char('e') if key.modifiers.ctrl => MinibufferEvent::MoveCursor(CursorDirection::End),
            KeyCode::Down => MinibufferEvent::CompletionNext,
            KeyCode::Up => MinibufferEvent::CompletionPrevious,
            _ => MinibufferEvent::Input('\0'), // 無効な入力として扱う
        }
    }

    /// 非アクティブ化
    pub fn deactivate(&mut self) {
        self.state.mode = MinibufferMode::Inactive;
        self.state.input.clear();
        self.state.completions.clear();
        self.state.selected_completion = None;
        self.state.cursor_pos = 0;
    }
}
```

### 補完システム

```rust
/// 補完エンジンのトレイト
pub trait CompletionEngine {
    /// 補完候補を取得
    fn get_completions(&self, input: &str, context: &CompletionContext) -> Vec<String>;

    /// 最適な補完を選択
    fn get_best_completion(&self, input: &str, candidates: &[String]) -> Option<String>;
}

/// 補完のコンテキスト
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// 補完の種類
    pub completion_type: CompletionType,
    /// 現在の作業ディレクトリ
    pub current_dir: PathBuf,
    /// 最大候補数（QA.mdの回答: 50個）
    pub max_candidates: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
    /// ファイルパス補完
    FilePath,
    /// コマンド補完
    Command,
}

/// ファイルパス補完エンジン
#[derive(Debug)]
pub struct FileCompletionEngine {
    /// キャッシュされた候補
    cache: HashMap<String, (Vec<String>, Instant)>,
    /// キャッシュの有効期限
    cache_duration: Duration,
}

impl FileCompletionEngine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_duration: Duration::from_secs(5),
        }
    }

    fn expand_tilde(&self, path: &str) -> String {
        if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                path.replacen('~', &home.to_string_lossy(), 1)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        }
    }

    fn get_directory_entries(&self, dir_path: &Path) -> Result<Vec<String>, std::io::Error> {
        let mut entries = Vec::new();

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();

            // 隠しファイルをスキップ（.で始まるファイル）
            if file_name.starts_with('.') && file_name.len() > 1 {
                continue;
            }

            if entry.file_type()?.is_dir() {
                entries.push(format!("{}/", file_name));
            } else {
                entries.push(file_name);
            }
        }

        entries.sort();
        Ok(entries)
    }
}

impl CompletionEngine for FileCompletionEngine {
    fn get_completions(&self, input: &str, context: &CompletionContext) -> Vec<String> {
        if context.completion_type != CompletionType::FilePath {
            return Vec::new();
        }

        let expanded_input = self.expand_tilde(input);
        let path = Path::new(&expanded_input);

        let (dir_path, file_prefix) = if expanded_input.ends_with('/') {
            (path, "")
        } else {
            match path.parent() {
                Some(parent) => (parent, path.file_name().unwrap_or_default().to_str().unwrap_or("")),
                None => (Path::new("."), &expanded_input),
            }
        };

        // ディレクトリエントリを取得
        let entries = match self.get_directory_entries(dir_path) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        // プレフィックスでフィルタリング
        let mut candidates: Vec<String> = entries
            .into_iter()
            .filter(|entry| entry.starts_with(file_prefix))
            .map(|entry| {
                if dir_path == Path::new(".") {
                    entry
                } else {
                    format!("{}/{}", dir_path.to_string_lossy(), entry)
                }
            })
            .collect();

        // QA.mdの回答に基づく上限（50個）
        candidates.truncate(context.max_candidates);
        candidates
    }

    fn get_best_completion(&self, input: &str, candidates: &[String]) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }

        // 最も長い共通プレフィックスを見つける
        let first = &candidates[0];
        let mut common_prefix = String::new();

        for (i, ch) in first.chars().enumerate() {
            if candidates.iter().all(|candidate| {
                candidate.chars().nth(i).map_or(false, |c| c == ch)
            }) {
                common_prefix.push(ch);
            } else {
                break;
            }
        }

        if common_prefix.len() > input.len() {
            Some(common_prefix)
        } else {
            Some(first.clone())
        }
    }
}
```

### 履歴管理

QA.mdの回答に基づき、セッション内のみの最小限実装。

```rust
/// セッション内履歴管理
#[derive(Debug, Clone)]
pub struct SessionHistory {
    /// 履歴エントリ
    entries: Vec<String>,
    /// 最大保存数
    max_entries: usize,
}

impl SessionHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 100, // セッション内最大100件
        }
    }

    /// エントリを追加
    pub fn add_entry(&mut self, entry: String) {
        // 重複を除去
        if let Some(pos) = self.entries.iter().position(|x| x == &entry) {
            self.entries.remove(pos);
        }

        // 先頭に追加
        self.entries.insert(0, entry);

        // 上限を超えた場合は古いものを削除
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }
    }

    /// 履歴を取得
    pub fn get_entry(&self, index: usize) -> Option<&String> {
        self.entries.get(index)
    }

    /// 履歴数を取得
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 空かどうか
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
```

## エラーハンドリング

### エラー種別

```rust
/// ミニバッファ関連のエラー
#[derive(Debug, thiserror::Error)]
pub enum MinibufferError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl MinibufferError {
    /// ユーザーフレンドリーなメッセージに変換
    pub fn user_message(&self) -> String {
        match self {
            MinibufferError::FileNotFound(path) => {
                format!("ファイルが見つかりません: {}", path)
            }
            MinibufferError::PermissionDenied(path) => {
                format!("アクセス権限がありません: {}", path)
            }
            MinibufferError::InvalidPath(path) => {
                format!("無効なパスです: {}", path)
            }
            MinibufferError::IoError(err) => {
                format!("ファイル操作エラー: {}", err)
            }
            MinibufferError::CommandNotFound(cmd) => {
                format!("コマンドが見つかりません: {}", cmd)
            }
            MinibufferError::InvalidInput(input) => {
                format!("無効な入力です: {}", input)
            }
        }
    }
}
```

## 描画システム

### TUI統合

```rust
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

impl Minibuffer {
    /// ミニバッファを描画
    pub fn render<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        match &self.state.mode {
            MinibufferMode::Inactive => {
                // 非アクティブ時は何も描画しない
            }
            MinibufferMode::ErrorDisplay { message, .. } => {
                self.render_message(f, area, message, self.style.error_style);
            }
            MinibufferMode::InfoDisplay { message, .. } => {
                self.render_message(f, area, message, self.style.input_style);
            }
            _ => {
                self.render_input_mode(f, area);
            }
        }
    }

    fn render_input_mode<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 入力行
                Constraint::Min(0),     // 補完候補
            ])
            .split(area);

        // 入力行を描画
        self.render_input_line(f, chunks[0]);

        // 補完候補を描画
        if !self.state.completions.is_empty() {
            self.render_completions(f, chunks[1]);
        }
    }

    fn render_input_line<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let input_text = format!("{}{}", self.state.prompt, self.state.input);
        let cursor_pos = self.state.prompt.len() + self.state.cursor_pos;

        let paragraph = Paragraph::new(input_text)
            .style(self.style.input_style)
            .block(Block::default().borders(Borders::ALL).border_style(self.style.border_style));

        f.render_widget(paragraph, area);

        // カーソル位置を設定
        if cursor_pos < area.width as usize {
            f.set_cursor(area.x + cursor_pos as u16, area.y);
        }
    }

    fn render_completions<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let max_display = std::cmp::min(10, area.height as usize); // 最大10行表示
        let display_completions = &self.state.completions[..std::cmp::min(max_display, self.state.completions.len())];

        let items: Vec<ListItem> = display_completions
            .iter()
            .enumerate()
            .map(|(i, completion)| {
                let style = if Some(i) == self.state.selected_completion {
                    self.style.selected_completion_style
                } else {
                    self.style.completion_style
                };
                ListItem::new(Span::styled(completion.clone(), style))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Completions"))
            .style(self.style.completion_style);

        f.render_widget(list, area);
    }

    fn render_message<B: Backend>(&self, f: &mut Frame<B>, area: Rect, message: &str, style: Style) {
        let paragraph = Paragraph::new(message)
            .style(style)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(paragraph, area);
    }
}
```

## キーバインド連携

### コマンド実行インターフェース

```rust
/// コマンド実行者のトレイト
pub trait CommandExecutor {
    /// コマンドを実行
    fn execute_command(&mut self, command: &str, args: &[String]) -> Result<String, MinibufferError>;

    /// 利用可能なコマンド一覧を取得
    fn get_available_commands(&self) -> Vec<String>;
}

/// ミニバッファとキーバインドの統合
impl Minibuffer {
    /// キーバインドシステムからの呼び出し
    pub fn handle_action(&mut self, action: MinibufferAction) -> MinibufferResult {
        match action {
            MinibufferAction::FindFile => {
                self.start_find_file(None);
                MinibufferResult::Continue
            }
            MinibufferAction::ExecuteCommand => {
                self.start_execute_command();
                MinibufferResult::Continue
            }
            MinibufferAction::SaveFile => {
                // 保存処理（実装は file operations で）
                self.show_info("File saved".to_string());
                MinibufferResult::Continue
            }
        }
    }

    fn submit(&mut self) -> MinibufferResult {
        let input = self.state.input.clone();

        match &self.state.mode {
            MinibufferMode::FindFile => {
                if input.is_empty() {
                    self.show_error("No file specified".to_string());
                    MinibufferResult::Continue
                } else {
                    // 履歴に追加
                    self.add_to_history(input.clone());
                    self.deactivate();
                    MinibufferResult::Execute(format!("find-file {}", input))
                }
            }
            MinibufferMode::ExecuteCommand => {
                if input.is_empty() {
                    self.show_error("No command specified".to_string());
                    MinibufferResult::Continue
                } else {
                    self.add_to_history(input.clone());
                    self.deactivate();
                    MinibufferResult::Execute(input)
                }
            }
            _ => MinibufferResult::Continue,
        }
    }

    fn cancel(&mut self) -> MinibufferResult {
        self.deactivate();
        MinibufferResult::Cancel
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferAction {
    FindFile,
    ExecuteCommand,
    SaveFile,
}
```

## パフォーマンス最適化

### 応答性の確保

```rust
impl Minibuffer {
    /// 補完候補の更新（非同期対応準備）
    fn update_completions(&mut self) {
        let context = CompletionContext {
            completion_type: match self.state.mode {
                MinibufferMode::FindFile => CompletionType::FilePath,
                MinibufferMode::ExecuteCommand => CompletionType::Command,
                _ => return,
            },
            current_dir: std::env::current_dir().unwrap_or_default(),
            max_candidates: 50, // QA.mdの回答
        };

        // 入力が短すぎる場合は補完しない（パフォーマンス考慮）
        if self.state.input.len() < 2 {
            self.state.completions.clear();
            self.state.selected_completion = None;
            return;
        }

        let completions = self.completion_engine.get_completions(&self.state.input, &context);
        self.state.completions = completions;
        self.state.selected_completion = if self.state.completions.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// メッセージの有効期限をチェック
    fn check_message_expiry(&mut self) {
        let now = Instant::now();

        match &self.state.mode {
            MinibufferMode::ErrorDisplay { expires_at, .. } |
            MinibufferMode::InfoDisplay { expires_at, .. } => {
                if now >= *expires_at {
                    self.deactivate();
                }
            }
            _ => {}
        }
    }
}
```

## テスト戦略

### ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minibuffer_creation() {
        let minibuffer = Minibuffer::new();
        assert!(matches!(minibuffer.state.mode, MinibufferMode::Inactive));
    }

    #[test]
    fn test_find_file_mode() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.start_find_file(Some("test.txt"));

        assert!(matches!(minibuffer.state.mode, MinibufferMode::FindFile));
        assert_eq!(minibuffer.state.input, "test.txt");
        assert_eq!(minibuffer.state.prompt, "Find file: ");
    }

    #[test]
    fn test_input_handling() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.start_find_file(None);

        let key = Key {
            modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
            code: KeyCode::Char('a'),
        };

        let result = minibuffer.handle_key(key);
        assert_eq!(result, MinibufferResult::Continue);
        assert_eq!(minibuffer.state.input, "a");
    }

    #[test]
    fn test_completion_engine() {
        let engine = FileCompletionEngine::new();
        let context = CompletionContext {
            completion_type: CompletionType::FilePath,
            current_dir: PathBuf::from("."),
            max_candidates: 50,
        };

        let completions = engine.get_completions("src/", &context);
        // 実際のファイルシステムに依存するため、結果の検証は環境依存
        println!("Completions: {:?}", completions);
    }

    #[test]
    fn test_session_history() {
        let mut history = SessionHistory::new();

        history.add_entry("file1.txt".to_string());
        history.add_entry("file2.txt".to_string());

        assert_eq!(history.len(), 2);
        assert_eq!(history.get_entry(0), Some(&"file2.txt".to_string()));
        assert_eq!(history.get_entry(1), Some(&"file1.txt".to_string()));
    }

    #[test]
    fn test_error_display() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.show_error("Test error".to_string());

        match &minibuffer.state.mode {
            MinibufferMode::ErrorDisplay { message, .. } => {
                assert_eq!(message, "Test error");
            }
            _ => panic!("Expected ErrorDisplay mode"),
        }
    }
}
```

### 統合テスト

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_file_open_workflow() {
        let mut minibuffer = Minibuffer::new();

        // ファイル検索開始
        minibuffer.start_find_file(None);

        // ファイル名入力
        let chars = "test.txt".chars();
        for ch in chars {
            let key = Key {
                modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
                code: KeyCode::Char(ch),
            };
            minibuffer.handle_key(key);
        }

        // Enter で確定
        let enter_key = Key {
            modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
            code: KeyCode::Enter,
        };
        let result = minibuffer.handle_key(enter_key);

        match result {
            MinibufferResult::Execute(command) => {
                assert_eq!(command, "find-file test.txt");
            }
            _ => panic!("Expected Execute result"),
        }

        assert!(matches!(minibuffer.state.mode, MinibufferMode::Inactive));
    }
}
```

## 将来の拡張

### プラグインシステム連携

```rust
/// プラグイン用のミニバッファ拡張
pub trait MinibufferExtension {
    /// カスタム補完を提供
    fn provide_completions(&self, input: &str, context: &CompletionContext) -> Vec<String>;

    /// カスタムコマンドを実行
    fn execute_custom_command(&self, command: &str, args: &[String]) -> Result<String, MinibufferError>;
}
```

### 設定システム連携

```rust
/// ミニバッファの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinibufferConfig {
    /// 最大補完候補数
    pub max_completions: usize,
    /// メッセージ表示時間（秒）
    pub message_display_duration: u64,
    /// 履歴最大保存数
    pub max_history_entries: usize,
    /// 補完の最小入力文字数
    pub min_completion_chars: usize,
    /// 外観設定
    pub style: MinibufferStyleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinibufferStyleConfig {
    pub prompt_color: String,
    pub input_color: String,
    pub completion_color: String,
    pub error_color: String,
}
```

## 制限事項

### MVPでの制約
- 履歴の永続化なし（セッション内のみ）
- 非同期補完未対応
- カスタムコマンド未対応
- プラグインシステム未対応

### 既知の制限
- ファイルシステムの権限に依存
- 大量のファイルがある場合の性能問題の可能性
- Unicode表示の端末依存

これらの制限は将来バージョンで段階的に解決予定。