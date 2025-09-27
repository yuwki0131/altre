//! メインアプリケーション構造体
//!
//! アプリケーション全体の状態管理とメインループを実装

use crate::buffer::{BufferManager, CursorPosition, EditOperations, NavigationAction, TextEditor};
use crate::error::{AltreError, Result, UiError};
use crate::input::keybinding::{ModernKeyMap, KeyProcessResult, Action, Key};
use crate::input::commands::{Command, CommandProcessor};
use crate::minibuffer::{MinibufferSystem, SystemEvent, SystemResponse};
use crate::ui::{AdvancedRenderer, ViewportState};
use crate::search::{SearchController, SearchDirection, SearchHighlight};
use crate::editor::KillRing;
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
    /// キルリング
    kill_ring: KillRing,
    /// 直前のキル関連コマンド
    kill_context: KillContext,
    /// 直近のヤンク範囲
    last_yank_range: Option<(usize, usize)>,
    /// 現在のビューポート状態
    viewport: ViewportState,
    /// `C-l` の再配置サイクル
    recenter_step: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillContext {
    None,
    Kill,
    Yank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillMerge {
    Append,
    Prepend,
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
            kill_ring: KillRing::new(),
            kill_context: KillContext::None,
            last_yank_range: None,
            viewport: ViewportState::new(),
            recenter_step: 0,
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
        // ミニバッファのメッセージ表示があれば先に消去
        if self.minibuffer.is_message_displayed() {
            let key = Key::from(key_event);
            if let Err(err) = self.minibuffer.handle_event(SystemEvent::KeyInput(key)) {
                self.show_error_message(AltreError::Application(format!(
                    "ミニバッファの処理に失敗しました: {}", err
                )));
                return Ok(());
            }
        }

        // ミニバッファがインタラクティブな場合の処理
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
                self.keyboard_quit();
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
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::DeleteBackwardChar => {
                if let Err(err) = self.editor.delete_backward() {
                    self.show_error_message(err);
                }
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::DeleteChar => {
                if let Err(err) = self.editor.delete_forward() {
                    self.show_error_message(err);
                }
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::InsertNewline => {
                if let Err(err) = self.editor.insert_newline() {
                    self.show_error_message(err);
                }
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::KillWordForward => {
                self.kill_word_forward();
                Ok(())
            }
            Command::KillWordBackward => {
                self.kill_word_backward();
                Ok(())
            }
            Command::KillLine => {
                self.kill_line_forward();
                Ok(())
            }
            Command::Yank => {
                self.yank();
                Ok(())
            }
            Command::YankPop => {
                self.yank_pop();
                Ok(())
            }
            Command::KeyboardQuit => {
                self.keyboard_quit();
                Ok(())
            }
            Command::SetMark => {
                self.set_mark_command();
                Ok(())
            }
            Command::KillRegion => self.kill_region(),
            Command::CopyRegion => self.copy_region(),
            Command::ExchangePointAndMark => self.exchange_point_and_mark(),
            Command::MarkBuffer => self.mark_entire_buffer(),
            Command::ScrollPageDown => {
                self.scroll_page_down();
                Ok(())
            }
            Command::ScrollPageUp => {
                self.scroll_page_up();
                Ok(())
            }
            Command::Recenter => {
                self.recenter_view();
                Ok(())
            }
            Command::ScrollLeft => {
                self.scroll_left();
                Ok(())
            }
            Command::ScrollRight => {
                self.scroll_right();
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

    fn record_kill(&mut self, text: String, merge: KillMerge) {
        if text.is_empty() {
            self.reset_kill_context();
            return;
        }

        match merge {
            KillMerge::Append => {
                if matches!(self.kill_context, KillContext::Kill) {
                    self.kill_ring.append_to_front(&text);
                } else {
                    self.kill_ring.push(text);
                }
            }
            KillMerge::Prepend => {
                if matches!(self.kill_context, KillContext::Kill) {
                    self.kill_ring.prepend_to_front(&text);
                } else {
                    self.kill_ring.push(text);
                }
            }
        }

        self.kill_context = KillContext::Kill;
        self.last_yank_range = None;
    }

    fn kill_word_forward(&mut self) {
        match self.editor.delete_word_forward() {
            Ok(text) => {
                self.record_kill(text, KillMerge::Append);
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => self.show_error_message(err),
        }
    }

    fn kill_word_backward(&mut self) {
        match self.editor.delete_word_backward() {
            Ok(text) => {
                self.record_kill(text, KillMerge::Prepend);
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => self.show_error_message(err),
        }
    }

    fn kill_line_forward(&mut self) {
        match self.editor.kill_line_forward() {
            Ok(text) => {
                self.record_kill(text, KillMerge::Append);
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => self.show_error_message(err),
        }
    }

    fn set_mark_command(&mut self) {
        self.editor.set_mark();
        self.show_info_message("マークを設定しました");
        self.reset_recenter_cycle();
    }

    fn kill_region(&mut self) -> Result<()> {
        if let Some((start, end)) = self.editor.selection_range() {
            let text = self.editor.delete_range_span(start, end)?;
            if text.is_empty() {
                self.show_info_message("選択範囲が空です");
            } else {
                self.record_kill(text, KillMerge::Append);
                self.kill_context = KillContext::Kill;
                self.last_yank_range = None;
            }
            self.editor.clear_mark();
            self.reset_recenter_cycle();
            self.ensure_cursor_visible();
        } else {
            self.show_info_message("リージョンが選択されていません");
        }
        Ok(())
    }

    fn copy_region(&mut self) -> Result<()> {
        if let Some((start, end)) = self.editor.selection_range() {
            let text = self.editor.get_text_range(start, end)?;
            if text.is_empty() {
                self.show_info_message("選択範囲が空です");
            } else {
                self.record_kill(text, KillMerge::Append);
                self.kill_context = KillContext::Kill;
                self.last_yank_range = None;
                self.show_info_message("リージョンをコピーしました");
            }
        } else {
            self.show_info_message("リージョンが選択されていません");
        }
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
        Ok(())
    }

    fn exchange_point_and_mark(&mut self) -> Result<()> {
        if self.editor.mark().is_none() {
            self.show_info_message("マークが設定されていません");
            return Ok(());
        }
        self.editor.swap_cursor_and_mark()?;
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
        Ok(())
    }

    fn mark_entire_buffer(&mut self) -> Result<()> {
        self.editor.mark_entire_buffer()?;
        self.show_info_message("バッファ全体を選択しました");
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
        Ok(())
    }

    fn yank(&mut self) {
        let Some(text) = self.kill_ring.yank() else {
            self.show_info_message("キルリングが空です");
            self.reset_kill_context();
            return;
        };

        let start = self.editor.cursor().char_pos;
        let len = text.chars().count();

        match self.editor.insert_str(&text) {
            Ok(_) => {
                self.kill_context = KillContext::Yank;
                self.last_yank_range = Some((start, len));
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => {
                self.reset_kill_context();
                self.show_error_message(err);
            }
        }
    }

    fn yank_pop(&mut self) {
        if !matches!(self.kill_context, KillContext::Yank) {
            self.show_info_message("直前のコマンドがヤンクではありません");
            self.reset_kill_context();
            return;
        }

        let Some((start, previous_len)) = self.last_yank_range else {
            self.show_info_message("ヤンク範囲が不明です");
            self.reset_kill_context();
            return;
        };

        if let Err(err) = self.editor.move_cursor_to_char(start) {
            self.reset_kill_context();
            self.show_error_message(err);
            return;
        }

        if let Err(err) = self.editor.delete_range(start, start + previous_len) {
            self.reset_kill_context();
            self.show_error_message(err);
            return;
        }

        let Some(next_text) = self.kill_ring.rotate() else {
            self.reset_kill_context();
            self.show_info_message("キルリングが空です");
            return;
        };

        let new_len = next_text.chars().count();
        if let Err(err) = self.editor.insert_str(&next_text) {
            self.reset_kill_context();
            self.show_error_message(err);
            return;
        }

        self.kill_context = KillContext::Yank;
        self.last_yank_range = Some((start, new_len));
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
    }

    fn keyboard_quit(&mut self) {
        self.reset_kill_context();
        self.reset_recenter_cycle();
        if self.search.is_active() {
            self.search.cancel(&mut self.editor);
        }
        self.editor.clear_mark();
        self.show_info_message("キャンセルしました");
        self.ensure_cursor_visible();
    }

    fn reset_kill_context(&mut self) {
        self.kill_context = KillContext::None;
        self.last_yank_range = None;
    }

    fn reset_recenter_cycle(&mut self) {
        self.recenter_step = 0;
    }

    fn buffer_metrics(&self) -> (usize, usize) {
        let content = self.editor.to_string();
        if content.is_empty() {
            return (1, 0);
        }

        let mut lines = 0usize;
        let mut max_columns = 0usize;
        for line in content.lines() {
            lines += 1;
            let columns = line.chars().count();
            if columns > max_columns {
                max_columns = columns;
            }
        }

        (lines.max(1), max_columns)
    }

    fn selection_highlights(&self) -> Vec<SearchHighlight> {
        let Some((start, end)) = self.editor.selection_range() else {
            return Vec::new();
        };

        if start == end {
            return Vec::new();
        }

        let (start_line, start_col) = self.editor.position_to_line_column(start);
        let (end_line, end_col) = self.editor.position_to_line_column(end);
        let text = self.editor.to_string();
        let lines: Vec<&str> = text.split('\n').collect();
        let mut highlights = Vec::new();

        let push_highlight = |line: usize, s: usize, e: usize, list: &mut Vec<SearchHighlight>| {
            if e > s {
                list.push(SearchHighlight {
                    line,
                    start_column: s,
                    end_column: e,
                    is_current: false,
                });
            }
        };

        if start_line == end_line {
            push_highlight(start_line, start_col, end_col, &mut highlights);
            return highlights;
        }

        let first_line_len = lines.get(start_line).map(|l| l.chars().count()).unwrap_or(0);
        push_highlight(start_line, start_col, first_line_len, &mut highlights);

        for line in (start_line + 1)..end_line {
            let len = lines.get(line).map(|l| l.chars().count()).unwrap_or(0);
            push_highlight(line, 0, len, &mut highlights);
        }

        push_highlight(end_line, 0, end_col, &mut highlights);

        highlights
    }

    fn ensure_cursor_visible(&mut self) {
        let (total_lines, max_columns) = self.buffer_metrics();
        self.viewport.clamp_vertical(total_lines);

        let height = self.viewport.height.max(1);
        let cursor_line = self.editor.cursor().line;
        if cursor_line < self.viewport.top_line {
            self.viewport.top_line = cursor_line;
        } else if cursor_line >= self.viewport.top_line + height {
            self.viewport.top_line = cursor_line + 1 - height;
        }

        self.viewport.clamp_vertical(total_lines);

        let cursor_column = self.editor.cursor().column;
        if cursor_column < self.viewport.scroll_x {
            self.viewport.scroll_x = cursor_column;
        } else if cursor_column >= self.viewport.scroll_x + self.viewport.width {
            self.viewport.scroll_x = cursor_column + 1 - self.viewport.width;
        }

        self.viewport.clamp_horizontal(max_columns);
    }

    fn move_cursor_vertical(&mut self, delta: isize) {
        if delta > 0 {
            for _ in 0..delta {
                match self.editor.navigate(NavigationAction::MoveLineDown) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        } else {
            for _ in 0..delta.unsigned_abs() {
                match self.editor.navigate(NavigationAction::MoveLineUp) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        }
    }

    fn move_cursor_horizontal(&mut self, delta: isize) {
        if delta > 0 {
            for _ in 0..delta {
                match self.editor.navigate(NavigationAction::MoveCharForward) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        } else {
            for _ in 0..delta.unsigned_abs() {
                match self.editor.navigate(NavigationAction::MoveCharBackward) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        }
    }

    fn scroll_page_down(&mut self) {
        let (total_lines, _) = self.buffer_metrics();
        let height = self.viewport.height.max(1);
        let step = height.saturating_sub(1).max(1);
        let old_top = self.viewport.top_line;
        let max_top = total_lines.saturating_sub(height);
        let new_top = (old_top + step).min(max_top);
        let delta = new_top.saturating_sub(old_top);
        self.viewport.top_line = new_top;
        if delta > 0 {
            self.move_cursor_vertical(delta as isize);
        }
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn scroll_page_up(&mut self) {
        let height = self.viewport.height.max(1);
        let step = height.saturating_sub(1).max(1);
        let old_top = self.viewport.top_line;
        let new_top = old_top.saturating_sub(step);
        let delta = old_top.saturating_sub(new_top);
        self.viewport.top_line = new_top;
        if delta > 0 {
            self.move_cursor_vertical(-(delta as isize));
        }
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn recenter_view(&mut self) {
        let (total_lines, _) = self.buffer_metrics();
        let height = self.viewport.height.max(1);
        let cursor_line = self.editor.cursor().line;
        let max_top = total_lines.saturating_sub(height);

        let desired_top = match self.recenter_step % 3 {
            0 => cursor_line.saturating_sub(height / 2),
            1 => cursor_line,
            _ => cursor_line.saturating_add(1).saturating_sub(height),
        };

        self.viewport.top_line = desired_top.min(max_top);
        self.recenter_step = (self.recenter_step + 1) % 3;
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn horizontal_scroll_step(&self) -> usize {
        (self.viewport.width / 2).max(1)
    }

    fn scroll_left(&mut self) {
        let step = self.horizontal_scroll_step();
        self.viewport.scroll_x = self.viewport.scroll_x.saturating_add(step);
        self.move_cursor_horizontal(step as isize);
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn scroll_right(&mut self) {
        let step = self.horizontal_scroll_step();
        if self.viewport.scroll_x > 0 {
            let delta = self.viewport.scroll_x.min(step);
            self.viewport.scroll_x -= delta;
            self.move_cursor_horizontal(-(delta as isize));
        }
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
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
                            self.viewport = ViewportState::new();
                            self.recenter_step = 0;
                            self.ensure_cursor_visible();
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
                            self.ensure_cursor_visible();
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
        self.reset_kill_context();
        self.reset_recenter_cycle();
        match self.editor.navigate(action) {
            Ok(true) => {
                self.ensure_cursor_visible();
            }
            Ok(false) => self.show_info_message("これ以上移動できません"),
            Err(err) => self.show_error_message(err.into()),
        }
    }

    fn render<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let search_ui = self.search.ui_state();
        let search_highlights = self.search.highlights();
        let selection_highlights = self.selection_highlights();
        let mut combined_highlights = Vec::with_capacity(search_highlights.len() + selection_highlights.len());
        combined_highlights.extend_from_slice(search_highlights);
        combined_highlights.extend(selection_highlights.into_iter());

        self.renderer
            .render(
                terminal,
                &self.editor,
                &mut self.viewport,
                &self.minibuffer,
                search_ui,
                &combined_highlights,
            )
            .map_err(|err| Self::terminal_error("render", err))
    }

    fn process_minibuffer_timer(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_line_removes_text_without_messages() {
        let mut app = App::new().expect("app init");
        app.insert_str("hello\nworld").unwrap();
        app.move_cursor_to_start().unwrap();

        app.handle_action(Action::KillLine).unwrap();

        assert_eq!(app.editor.to_string(), "world");
        assert_eq!(app.viewport.top_line, 0);
        assert_eq!(app.viewport.scroll_x, 0);
    }
}
