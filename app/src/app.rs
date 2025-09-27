//! メインアプリケーション構造体
//!
//! アプリケーション全体の状態管理とメインループを実装

use crate::buffer::{BufferManager, CursorPosition, EditOperations, NavigationAction, TextEditor};
use crate::error::{AltreError, Result, UiError};
use crate::input::keybinding::{ModernKeyMap, KeyProcessResult, Action};
use crate::input::commands::{Command, CommandProcessor};
use crate::minibuffer::MinibufferSystem;
use crate::ui::AdvancedRenderer;
use crate::search::{SearchController, SearchDirection};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io::stdout;
use std::path::Path;
use std::time::Duration;

/// デバッグ出力マクロ
macro_rules! debug_log {
    ($app:expr, $($arg:tt)*) => {
        if $app.debug_mode {
            eprintln!("DEBUG: {}", format!($($arg)*));
        }
    };
}

/// メインアプリケーション構造体
///
/// 全てのコンポーネントを統合し、アプリケーションのライフサイクルを管理
pub struct App {
    /// アプリケーション実行状態
    running: bool,
    /// 初期化状態
    initialized: bool,
    /// バッファマネージャー
    buffer_manager: BufferManager,
    /// メインエディタ
    editor: TextEditor,
    /// ミニバッファシステム
    minibuffer: MinibufferSystem,
    /// 高性能レンダラー
    renderer: AdvancedRenderer,
    /// キーマップ
    keymap: ModernKeyMap,
    /// コマンドプロセッサー
    command_processor: CommandProcessor,
    /// 検索コントローラ
    search: SearchController,
    /// 現在のプレフィックスキー状態
    current_prefix: Option<String>,
    /// デバッグモード
    debug_mode: bool,
}

impl App {
    /// 新しいアプリケーションインスタンスを作成
    pub fn new() -> Result<Self> {
        Ok(App {
            running: true,
            initialized: true,
            buffer_manager: BufferManager::new(),
            editor: TextEditor::new(),
            minibuffer: MinibufferSystem::new(),
            renderer: AdvancedRenderer::new(),
            keymap: ModernKeyMap::new(),
            command_processor: CommandProcessor::new(),
            search: SearchController::new(),
            current_prefix: None,
            debug_mode: std::env::var("ALTRE_DEBUG").is_ok(),
        })
    }

    /// メインイベントループを実行
    pub fn run(&mut self) -> Result<()> {
        self.enter_terminal()?;

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)
            .map_err(|err| Self::terminal_error("terminal init", err))?;
        terminal
            .hide_cursor()
            .map_err(|err| Self::terminal_error("hide cursor", err))?;

        let loop_result = self.event_loop(&mut terminal);
        let show_cursor_result = terminal
            .show_cursor()
            .map_err(|err| Self::terminal_error("show cursor", err));
        drop(terminal);
        let cleanup_result = self.leave_terminal();

        loop_result
            .and(show_cursor_result)
            .and(cleanup_result)
    }

    /// アプリケーションが実行中かどうかを確認
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// アプリケーションを終了状態にする
    pub fn shutdown(&mut self) {
        self.running = false;
    }

    /// アプリケーションが初期化されているかを確認
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// ファイルを開く
    pub fn open_file(&mut self, file_path: &str) -> Result<()> {
        let _buffer_id = self.buffer_manager.create_buffer();

        if Path::new(file_path).exists() {
            // TODO: 実際のファイル読み込みを実装
            self.show_info_message(format!("未実装: {} の読み込み", file_path));
        } else {
            self.show_error_message(AltreError::Application(format!(
                "ファイルが存在しません: {}",
                file_path
            )));
        }

        Ok(())
    }

    /// バッファが存在するかを確認
    pub fn has_buffer(&self) -> bool {
        self.buffer_manager.current_buffer_id().is_some()
    }

    /// 文字を挿入
    pub fn insert_char(&mut self, ch: char) -> Result<()> {
        self.editor.insert_char(ch)
    }

    /// バッファの内容を取得
    pub fn get_buffer_content(&self) -> String {
        self.editor.to_string()
    }

    /// 文字列を挿入
    pub fn insert_str(&mut self, s: &str) -> Result<()> {
        self.editor.insert_str(s)
    }

    /// カーソルを開始位置に移動
    pub fn move_cursor_to_start(&mut self) -> Result<()> {
        let start_position = CursorPosition::new();
        self.editor.set_cursor(start_position);
        Ok(())
    }

    /// カーソル位置を取得
    pub fn get_cursor_position(&self) -> &CursorPosition {
        self.editor.cursor()
    }

    fn event_loop<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while self.running {
            self.process_minibuffer_timer();
            self.render(terminal)?;

            if event::poll(Duration::from_millis(16))
                .map_err(|err| Self::terminal_error("event poll", err))?
            {
                match event::read().map_err(|err| Self::terminal_error("event read", err))? {
                    Event::Key(key_event) => self.handle_key_event(key_event)?,
                    Event::Resize(_, _) => {
                        // 次回描画で自動的に反映されるため処理不要
                    }
                    Event::Mouse(_) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
                }
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        // ミニバッファがアクティブな場合の処理
        if self.minibuffer.is_active() {
            return self.handle_minibuffer_key(key_event);
        }

        // 検索モードがアクティブな場合は専用処理
        if self.search.is_active() {
            self.handle_search_key(key_event);
            return Ok(());
        }

        // 検索開始キー（C-s/C-r）を優先的に処理
        if self.try_start_search(&key_event) {
            return Ok(());
        }

        // 特殊キー処理（C-g, ESCなど）
        if self.handle_special_keys(&key_event) {
            return Ok(());
        }

        // 新しいキーマップシステムを使用してキーを処理
        let result = self.keymap.process_key_event(key_event);

        match result {
            KeyProcessResult::Action(action) => {
                // アクション実行時にプレフィックス状態をクリア
                self.current_prefix = None;
                self.handle_action(action)?;
            }
            KeyProcessResult::PartialMatch => {
                // プレフィックスキーの場合、状態を記録（ミニバッファは使わない）
                if let Some(prefix) = self.keymap.current_prefix_label() {
                    self.current_prefix = Some(prefix.to_string());
                } else {
                    self.current_prefix = None;
                }
            }
            KeyProcessResult::NoMatch => {
                // 緊急終了のフォールバック
                if key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c') {
                    self.shutdown();
                } else {
                    self.show_info_message(format!("未対応のキー: {}", Self::format_key_event(&key_event)));
                }
            }
        }

        Ok(())
    }

    /// 特殊キーの処理（キーマップを迂回）
    fn handle_special_keys(&mut self, key_event: &KeyEvent) -> bool {
        match (key_event.code, key_event.modifiers) {
            // C-g: キーシーケンスのキャンセル（無反応）
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.keymap.reset_partial_match();
                self.current_prefix = None;
                true
            }
            // ESC: キーシーケンスのキャンセル（無反応）
            (KeyCode::Esc, _) => {
                self.keymap.reset_partial_match();
                self.current_prefix = None;
                true
            }
            _ => false,
        }
    }

    fn try_start_search(&mut self, key_event: &KeyEvent) -> bool {
        if self.keymap.is_partial_match() {
            return false;
        }

        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            match key_event.code {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.keymap.reset_partial_match();
                    self.current_prefix = None;
                    self.search.start(&mut self.editor, SearchDirection::Forward);
                    return true;
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.keymap.reset_partial_match();
                    self.current_prefix = None;
                    self.search.start(&mut self.editor, SearchDirection::Backward);
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    fn handle_search_key(&mut self, key_event: KeyEvent) {
        use KeyModifiers as KM;

        let modifiers = key_event.modifiers;

        match key_event.code {
            KeyCode::Char('s') | KeyCode::Char('S') if modifiers.contains(KM::CONTROL) => {
                self.search.repeat_forward(&mut self.editor);
            }
            KeyCode::Char('r') | KeyCode::Char('R') if modifiers.contains(KM::CONTROL) => {
                self.search.repeat_backward(&mut self.editor);
            }
            KeyCode::Char('w') | KeyCode::Char('W') if modifiers.contains(KM::CONTROL) => {
                self.search.add_word_at_cursor(&mut self.editor);
            }
            KeyCode::Char('g') | KeyCode::Char('G') if modifiers.contains(KM::CONTROL) => {
                self.search.cancel(&mut self.editor);
            }
            KeyCode::Enter => {
                self.search.accept();
            }
            KeyCode::Backspace => {
                self.search.delete_char(&mut self.editor);
            }
            KeyCode::Esc => {
                self.search.cancel(&mut self.editor);
            }
            KeyCode::Char(ch) => {
                if modifiers.contains(KM::CONTROL) || modifiers.contains(KM::ALT) {
                    // 未対応の制御入力はキャンセル扱い
                    if modifiers.contains(KM::CONTROL) && (ch == 'g' || ch == 'G') {
                        self.search.cancel(&mut self.editor);
                    }
                } else {
                    self.search.input_char(&mut self.editor, ch);
                }
            }
            _ => {}
        }
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        match action.to_command() {
            Some(command) => self.execute_command(command),
            None => {
                self.show_error_message(AltreError::Application(
                    "アクションをコマンドに変換できませんでした".to_string()
                ));
                Ok(())
            }
        }
    }

    fn execute_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::FindFile => {
                self.start_find_file_prompt()
            }
            Command::ForwardChar => {
                self.navigate(NavigationAction::MoveCharForward);
                Ok(())
            }
            Command::BackwardChar => {
                self.navigate(NavigationAction::MoveCharBackward);
                Ok(())
            }
            Command::NextLine => {
                self.navigate(NavigationAction::MoveLineDown);
                Ok(())
            }
            Command::PreviousLine => {
                self.navigate(NavigationAction::MoveLineUp);
                Ok(())
            }
            Command::InsertChar(ch) => {
                if let Err(err) = self.editor.insert_char(ch) {
                    self.show_error_message(err);
                }
                Ok(())
            }
            Command::DeleteBackwardChar => {
                if let Err(err) = self.editor.delete_backward() {
                    self.show_error_message(err);
                }
                Ok(())
            }
            Command::DeleteChar => {
                if let Err(err) = self.editor.delete_forward() {
                    self.show_error_message(err);
                }
                Ok(())
            }
            Command::InsertNewline => {
                if let Err(err) = self.editor.insert_newline() {
                    self.show_error_message(err);
                }
                Ok(())
            }
            Command::SaveBuffer => {
                match self.command_processor.current_buffer() {
                    Some(buffer) => {
                        if buffer.path.is_none() {
                            let suggested = if buffer.name.trim().is_empty() {
                                "untitled".to_string()
                            } else {
                                buffer.name.clone()
                            };
                            self.start_save_as_prompt(&suggested)?;
                        } else {
                            let result = self.command_processor.execute(Command::SaveBuffer);
                            if result.success {
                                if let Some(msg) = result.message {
                                    self.show_info_message(msg);
                                }
                            } else if let Some(msg) = result.message {
                                self.show_error_message(AltreError::Application(msg));
                            }
                        }
                    }
                    None => {
                        // ファイルが開かれていない場合、新規ファイル名を入力するためのミニバッファを起動
                        let current_dir = std::env::current_dir()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| "~/".to_string());

                        let initial_path = if current_dir.ends_with('/') {
                            format!("{}untitled", current_dir)
                        } else {
                            format!("{}/untitled", current_dir)
                        };

                        self.start_save_as_prompt(&initial_path)?;
                    }
                }
                Ok(())
            }
            Command::SaveBuffersKillTerminal | Command::Quit => {
                self.shutdown();
                Ok(())
            }
            Command::ExecuteCommand => {
                self.start_execute_command_prompt()
            }
            Command::EvalExpression => {
                self.start_eval_expression_prompt()
            }
            Command::MoveLineStart => {
                self.navigate(NavigationAction::MoveLineStart);
                Ok(())
            }
            Command::MoveLineEnd => {
                self.navigate(NavigationAction::MoveLineEnd);
                Ok(())
            }
            Command::MoveBufferStart => {
                self.navigate(NavigationAction::MoveBufferStart);
                Ok(())
            }
            Command::MoveBufferEnd => {
                self.navigate(NavigationAction::MoveBufferEnd);
                Ok(())
            }
            _ => {
                self.show_info_message(format!("未実装のコマンド: {:?}", command));
                Ok(())
            }
        }
    }

    fn start_find_file_prompt(&mut self) -> Result<()> {
        // カレントディレクトリを取得
        let current_dir = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/".to_string());

        // ディレクトリパスに末尾スラッシュを追加
        let initial_path = if current_dir.ends_with('/') {
            current_dir
        } else {
            format!("{}/", current_dir)
        };

        // ミニバッファでファイル検索を開始
        match self.minibuffer.start_find_file(Some(&initial_path)) {
            Ok(_) => {
                Ok(())
            },
            Err(err) => {
                self.show_error_message(AltreError::Application(format!(
                    "ミニバッファの初期化に失敗しました: {}", err
                )));
                Ok(())
            }
        }
    }

    fn start_execute_command_prompt(&mut self) -> Result<()> {
        match self.minibuffer.start_execute_command() {
            Ok(_) => Ok(()),
            Err(err) => {
                self.show_error_message(AltreError::Application(format!(
                    "ミニバッファの初期化に失敗しました: {}", err
                )));
                Ok(())
            }
        }
    }

    fn start_eval_expression_prompt(&mut self) -> Result<()> {
        match self.minibuffer.start_eval_expression() {
            Ok(_) => Ok(()),
            Err(err) => {
                self.show_error_message(AltreError::Application(format!(
                    "ミニバッファの初期化に失敗しました: {}", err
                )));
                Ok(())
            }
        }
    }

    fn start_save_as_prompt(&mut self, suggested_name: &str) -> Result<()> {
        let initial_path = env::current_dir()
            .map(|dir| dir.join(suggested_name))
            .unwrap_or_else(|_| std::path::PathBuf::from(suggested_name.to_string()));

        let initial_string = initial_path.display().to_string();

        match self
            .minibuffer
            .start_write_file(Some(initial_string.as_str()))
        {
            Ok(_) => Ok(()),
            Err(err) => {
                self.show_error_message(AltreError::Application(format!(
                    "ミニバッファの初期化に失敗しました: {}", err
                )));
                Ok(())
            }
        }
    }

    fn handle_minibuffer_key(&mut self, key_event: KeyEvent) -> Result<()> {
        use crate::input::keybinding::Key;
        use crate::minibuffer::{SystemEvent, SystemResponse};

        let key: Key = key_event.into();

        match self.minibuffer.handle_event(SystemEvent::KeyInput(key)) {
            Ok(SystemResponse::FileOperation(file_op)) => {
                use crate::minibuffer::FileOperation;
                match file_op {
                    FileOperation::Open(path) => {
                        // ファイルを開く
                        debug_log!(self, "Opening file: {}", path);
                        let result = self.command_processor.open_file(path.clone());
                        if result.success {
                            if let Some(msg) = result.message {
                                self.show_info_message(msg);
                            }
                            // エディタの内容を同期（TODO: より良い統合方法を検討）
                            let editor_content = self.command_processor.editor().to_string();
                            self.editor = crate::buffer::TextEditor::from_str(&editor_content);
                            debug_log!(self, "File opened successfully, editor synchronized");
                        } else {
                            if let Some(msg) = result.message {
                                self.show_error_message(AltreError::Application(msg));
                            }
                        }
                    }
                    FileOperation::SaveAs(path) => {
                        // 現在のエディタ内容を同期
                        let editor_content = self.editor.to_string();
                        self.command_processor.sync_editor_content(&editor_content);

                        let result = self.command_processor.save_buffer_as(path.clone());
                        if result.success {
                            if let Some(msg) = result.message {
                                self.show_info_message(msg);
                            }
                        } else if let Some(msg) = result.message {
                            self.show_error_message(AltreError::Application(msg));
                        }
                    }
                    _ => {
                        self.show_info_message("未実装のファイル操作です");
                    }
                }
                Ok(())
            }
            Ok(SystemResponse::ExecuteCommand(cmd)) => {
                self.show_info_message(format!("コマンド実行: {}", cmd));
                Ok(())
            }
            Ok(SystemResponse::Quit) => {
                self.shutdown();
                Ok(())
            }
            Ok(SystemResponse::Continue) | Ok(SystemResponse::None) => {
                // 継続または何もしない
                Ok(())
            }
            Err(err) => {
                self.show_error_message(AltreError::Application(format!(
                    "ミニバッファエラー: {}", err
                )));
                Ok(())
            }
        }
    }

    fn navigate(&mut self, action: NavigationAction) {
        match self.editor.navigate(action) {
            Ok(true) => {}
            Ok(false) => self.show_info_message("これ以上移動できません"),
            Err(err) => self.show_error_message(err.into()),
        }
    }

    fn render<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let search_ui = self.search.ui_state();
        let highlights = self.search.highlights();

        self.renderer
            .render(terminal, &self.editor, &self.minibuffer, search_ui, highlights)
            .map_err(|err| Self::terminal_error("render", err))
    }

    fn process_minibuffer_timer(&mut self) {
        use crate::minibuffer::SystemEvent;
        if let Err(err) = self.minibuffer.handle_event(SystemEvent::Update) {
            eprintln!("minibuffer update error: {}", err);
        }
    }

    fn show_info_message<S: Into<String>>(&mut self, message: S) {
        if let Err(err) = self.minibuffer.show_info(message.into()) {
            eprintln!("minibuffer info error: {}", err);
        }
    }

    fn show_error_message(&mut self, error: AltreError) {
        if let Err(mini_err) = self.minibuffer.show_error(error.to_string()) {
            eprintln!("minibuffer error: {}", mini_err);
        }
    }

    fn enter_terminal(&self) -> Result<()> {
        enable_raw_mode().map_err(|err| Self::terminal_error("enable raw mode", err))?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)
            .map_err(|err| Self::terminal_error("enter alternate screen", err))?;
        Ok(())
    }

    fn leave_terminal(&self) -> Result<()> {
        let mut out = stdout();
        execute!(out, LeaveAlternateScreen)
            .map_err(|err| Self::terminal_error("leave alternate screen", err))?;
        disable_raw_mode().map_err(|err| Self::terminal_error("disable raw mode", err))?;
        Ok(())
    }

    fn terminal_error(context: &str, err: impl std::fmt::Display) -> AltreError {
        AltreError::Ui(UiError::RenderingFailed {
            component: format!("{}: {}", context, err),
        })
    }

    /// キーイベントを人間が読みやすい形式に変換
    fn format_key_event(key_event: &KeyEvent) -> String {
        let mut parts = Vec::new();

        // 修飾キーを追加（Shiftは特殊文字以外では通常表示しない）
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("C");
        }
        if key_event.modifiers.contains(KeyModifiers::ALT) {
            parts.push("M");
        }

        // 基本キーを追加
        let key_name = match key_event.code {
            KeyCode::Char(c) => {
                if c.is_ascii_control() {
                    // 制御文字の場合
                    format!("C-{}", (c as u8 + b'A' - 1) as char)
                } else if c.is_uppercase() && key_event.modifiers.contains(KeyModifiers::SHIFT) {
                    // 大文字のShift表示
                    format!("S-{}", c.to_lowercase())
                } else {
                    // 通常の文字
                    c.to_string()
                }
            }
            KeyCode::F(n) => format!("F{}", n),
            KeyCode::Enter => "RET".to_string(),
            KeyCode::Left => "左".to_string(),
            KeyCode::Right => "右".to_string(),
            KeyCode::Up => "上".to_string(),
            KeyCode::Down => "下".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Tab => "TAB".to_string(),
            KeyCode::BackTab => "S-TAB".to_string(),
            KeyCode::Delete => "DEL".to_string(),
            KeyCode::Insert => "INS".to_string(),
            KeyCode::Esc => "ESC".to_string(),
            KeyCode::Backspace => "BS".to_string(),
            KeyCode::CapsLock => "CapsLock".to_string(),
            KeyCode::ScrollLock => "ScrollLock".to_string(),
            KeyCode::NumLock => "NumLock".to_string(),
            KeyCode::PrintScreen => "PrintScreen".to_string(),
            KeyCode::Pause => "Pause".to_string(),
            KeyCode::Menu => "Menu".to_string(),
            KeyCode::KeypadBegin => "Keypad-Begin".to_string(),
            _ => format!("未知のキー"),
        };

        if parts.is_empty() {
            key_name
        } else {
            format!("{}-{}", parts.join("-"), key_name)
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("アプリケーションの初期化に失敗しました")
    }
}
