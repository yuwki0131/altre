//! メインアプリケーション構造体
//!
//! アプリケーション全体の状態管理とメインループを実装

use crate::buffer::{CursorPosition, EditOperations, NavigationAction, TextEditor};
use crate::error::{AltreError, Result, UiError, FileError};
use crate::input::keybinding::{ModernKeyMap, KeyProcessResult, Action, Key};
use crate::input::commands::{Command, CommandProcessor};
use crate::minibuffer::{MinibufferSystem, MinibufferAction, SystemEvent, SystemResponse};
use crate::ui::{AdvancedRenderer, StatusLineInfo, WindowManager, SplitOrientation, ViewportState};
use crate::search::{HighlightKind, QueryReplaceController, ReplaceProgress, ReplaceSummary, SearchController, SearchDirection, SearchHighlight};
use crate::editor::{KillRing, HistoryManager, HistoryStack, HistoryCommandKind, edit_utils};
use crate::file::{operations::FileOperationManager, FileBuffer, expand_path};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_TAB_WIDTH: usize = 4;

/// デバッグ出力マクロ
macro_rules! debug_log {
    ($app:expr, $($arg:tt)*) => {
        if $app.debug_mode {
            eprintln!("DEBUG: {}", format!($($arg)*));
        }
    };
}

#[derive(Clone)]
struct OpenBuffer {
    id: usize,
    file: FileBuffer,
    cursor: CursorPosition,
    history: HistoryStack,
}

impl OpenBuffer {
    fn new(id: usize, file: FileBuffer) -> Self {
        Self {
            id,
            cursor: CursorPosition::new(),
            file,
            history: HistoryStack::new(),
        }
    }

    fn name(&self) -> &str {
        &self.file.name
    }

    fn path(&self) -> Option<&PathBuf> {
        self.file.path.as_ref()
    }

    fn is_modified(&self) -> bool {
        self.file.is_modified()
    }
}

#[derive(Default)]
struct ReplaceSession {
    controller: QueryReplaceController,
    highlights: Vec<SearchHighlight>,
}

impl ReplaceSession {
    fn new() -> Self {
        Self {
            controller: QueryReplaceController::new(),
            highlights: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.controller = QueryReplaceController::new();
        self.highlights.clear();
    }
}

/// メインアプリケーション構造体
///
/// 全てのコンポーネントを統合し、アプリケーションのライフサイクルを管理
pub struct App {
    /// アプリケーション実行状態
    running: bool,
    /// 初期化状態
    initialized: bool,
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
    /// 置換セッション
    replace: ReplaceSession,
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
    /// ウィンドウ管理
    window_manager: WindowManager,
    /// 開いているバッファ一覧
    buffers: Vec<OpenBuffer>,
    /// 現在アクティブなバッファID
    current_buffer_id: Option<usize>,
    /// 直前にアクティブだったバッファID
    last_buffer_id: Option<usize>,
    /// バッファID払い出し用カウンタ
    next_buffer_id: usize,
    /// `C-l` の再配置サイクル
    recenter_step: u8,
    /// Undo/Redo 管理
    history: HistoryManager,
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
        let mut app = App {
            running: true,
            initialized: true,
            editor: TextEditor::new(),
            minibuffer: MinibufferSystem::new(),
            renderer: AdvancedRenderer::new(),
            keymap: ModernKeyMap::new(),
            command_processor: CommandProcessor::new(),
            search: SearchController::new(),
            replace: ReplaceSession::new(),
            current_prefix: None,
            debug_mode: std::env::var("ALTRE_DEBUG").is_ok(),
            kill_ring: KillRing::new(),
            kill_context: KillContext::None,
            last_yank_range: None,
            window_manager: WindowManager::new(),
            buffers: Vec::new(),
            current_buffer_id: None,
            last_buffer_id: None,
            next_buffer_id: 0,
            recenter_step: 0,
            history: HistoryManager::new(),
        };
        app.history.bind_editor(&mut app.editor);

        app.initialize_default_buffer()?;

        Ok(app)
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
        let message = self.open_file_at_path(file_path)?;
        self.show_info_message(message);
        Ok(())
    }

    /// バッファが存在するかを確認
    pub fn has_buffer(&self) -> bool {
        !self.buffers.is_empty()
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

    /// 次の単語末尾へ移動（M-f相当、テスト支援用）
    pub fn move_word_forward(&mut self) -> Result<bool> {
        self.editor
            .navigate(NavigationAction::MoveWordForward)
            .map_err(|err| err.into())
    }

    /// 前の単語先頭へ移動（M-b相当、テスト支援用）
    pub fn move_word_backward(&mut self) -> Result<bool> {
        self.editor
            .navigate(NavigationAction::MoveWordBackward)
            .map_err(|err| err.into())
    }

    /// カーソル位置を取得
    pub fn get_cursor_position(&self) -> &CursorPosition {
        self.editor.cursor()
    }

    fn initialize_default_buffer(&mut self) -> Result<()> {
        let id = self.allocate_buffer_id();
        let file_buffer = FileBuffer::new_empty("*scratch*".to_string());
        self.buffers.push(OpenBuffer::new(id, file_buffer));
        self.current_buffer_id = Some(id);
        self.load_buffer_by_id(id, false)?;
        Ok(())
    }

    fn allocate_buffer_id(&mut self) -> usize {
        let id = self.next_buffer_id;
        self.next_buffer_id = self.next_buffer_id.saturating_add(1);
        id
    }

    fn find_buffer_index(&self, id: usize) -> Option<usize> {
        self.buffers.iter().position(|buffer| buffer.id == id)
    }

    fn find_buffer_index_by_name(&self, name: &str) -> Option<usize> {
        self.buffers.iter().position(|buffer| buffer.name() == name)
    }

    fn find_buffer_id_by_path(&self, path: &Path) -> Option<usize> {
        self.buffers
            .iter()
            .find(|buffer| buffer.path().map_or(false, |p| p == path))
            .map(|buffer| buffer.id)
    }

    fn current_buffer_index(&self) -> Option<usize> {
        self.current_buffer_id
            .and_then(|id| self.find_buffer_index(id))
    }

    fn current_buffer(&self) -> Option<&OpenBuffer> {
        if let Some(id) = self.current_buffer_id {
            if let Some(index) = self.find_buffer_index(id) {
                return self.buffers.get(index);
            }
        }
        None
    }

    pub fn current_buffer_name(&self) -> Option<String> {
        self.current_buffer().map(|buffer| buffer.name().to_string())
    }

    fn persist_current_buffer_state(&mut self) {
        if let Some(index) = self.current_buffer_index() {
            if let Some(buffer) = self.buffers.get_mut(index) {
                buffer.file.content = self.editor.to_string();
                buffer.cursor = *self.editor.cursor();
                buffer.history = self.history.stack().clone();
            }
        }
    }

    fn load_buffer_by_id(&mut self, id: usize, persist_current: bool) -> Result<()> {
        if self.current_buffer_id == Some(id) {
            return Ok(());
        }

        if persist_current {
            self.persist_current_buffer_state();
        }

        let index = self.find_buffer_index(id)
            .ok_or_else(|| AltreError::Application(format!("バッファID {} が見つかりません", id)))?;

        let (content, cursor, file_clone, history_clone) = {
            let buffer = &self.buffers[index];
            (
                buffer.file.content.clone(),
                buffer.cursor,
                buffer.file.clone(),
                buffer.history.clone(),
            )
        };

        if let Some(current_id) = self.current_buffer_id {
            if current_id != id {
                self.last_buffer_id = Some(current_id);
            }
        }

        self.current_buffer_id = Some(id);
        self.editor = TextEditor::from_str(&content);
        self.editor.set_cursor(cursor);
        self.history.replace_stack(history_clone, &mut self.editor);
        self.command_processor.set_current_buffer(file_clone);
        self.command_processor.sync_editor_content(&self.editor.to_string());

        if let Some(viewport) = self.window_manager.focused_viewport_mut() {
            *viewport = ViewportState::new();
        }

        self.recenter_step = 0;
        self.ensure_cursor_visible();
        Ok(())
    }

    pub fn buffer_names(&self) -> Vec<String> {
        self.buffers.iter().map(|buffer| buffer.name().to_string()).collect()
    }

    fn buffer_display_lines(&self) -> Vec<String> {
        self.buffers
            .iter()
            .map(|buffer| {
                let mut markers = String::new();
                if Some(buffer.id) == self.current_buffer_id {
                    markers.push('*');
                } else {
                    markers.push(' ');
                }

                if buffer.is_modified() {
                    markers.push('!');
                } else {
                    markers.push(' ');
                }

                let path = buffer
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "[未保存]".to_string());

                format!("{} {:<20} {}", markers, buffer.name(), path)
            })
            .collect()
    }

    fn last_buffer_name(&self) -> Option<String> {
        self.last_buffer_id
            .and_then(|id| self.find_buffer_index(id))
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.name().to_string())
    }

    pub fn switch_buffer(&mut self, name: &str) -> Result<()> {
        self.switch_to_buffer_by_name(name)
    }

    pub fn kill_buffer(&mut self, name: Option<&str>) -> Result<()> {
        self.kill_buffer_by_name(name)
    }

    fn switch_to_buffer_by_name(&mut self, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            self.show_error_message(AltreError::Application("バッファ名を入力してください".to_string()));
            return Ok(());
        }

        let index = self.find_buffer_index_by_name(name)
            .ok_or_else(|| AltreError::Application(format!("バッファ '{}' が見つかりません", name)))?;
        let target_id = self.buffers[index].id;
        self.load_buffer_by_id(target_id, true)?;
        self.show_info_message(format!("バッファを切り替えました: {}", name));
        Ok(())
    }

    fn kill_buffer_by_name(&mut self, name: Option<&str>) -> Result<()> {
        if self.buffers.len() <= 1 {
            self.show_error_message(AltreError::Application("最後のバッファは削除できません".to_string()));
            return Ok(());
        }

        let target_id = if let Some(buffer_name) = name.filter(|n| !n.trim().is_empty()) {
            let index = self.find_buffer_index_by_name(buffer_name)
                .ok_or_else(|| AltreError::Application(format!("バッファ '{}' が見つかりません", buffer_name)))?;
            self.buffers[index].id
        } else {
            self.current_buffer_id
                .ok_or_else(|| AltreError::Application("カレントバッファが存在しません".to_string()))?
        };

        let index = self.find_buffer_index(target_id)
            .ok_or_else(|| AltreError::Application("指定されたバッファが見つかりません".to_string()))?;

        if self.buffers[index].is_modified() {
            let name = self.buffers[index].name().to_string();
            self.show_error_message(AltreError::Application(format!(
                "バッファ '{}' は未保存の変更があります", name
            )));
            return Ok(());
        }

        let removing_current = self.current_buffer_id == Some(target_id);
        if removing_current {
            self.persist_current_buffer_state();
        }

        let removed_name = self.buffers[index].name().to_string();
        self.buffers.remove(index);

        if self.last_buffer_id == Some(target_id) {
            self.last_buffer_id = None;
        }

        if removing_current {
            self.current_buffer_id = None;

            let fallback_id = self
                .last_buffer_id
                .and_then(|id| self.find_buffer_index(id))
                .and_then(|idx| self.buffers.get(idx))
                .map(|buffer| buffer.id)
                .or_else(|| self.buffers.first().map(|buffer| buffer.id))
                .ok_or_else(|| AltreError::Application("他のバッファが存在しません".to_string()))?;

            self.load_buffer_by_id(fallback_id, false)?;
        }

        self.show_info_message(format!("バッファを削除しました: {}", removed_name));
        Ok(())
    }

    fn show_buffer_list(&mut self) {
        let lines = self.buffer_display_lines();
        if lines.is_empty() {
            self.show_info_message("バッファはありません");
        } else {
            self.show_info_message(lines.join("\n"));
        }
    }

    fn open_file_at_path(&mut self, path_input: &str) -> Result<String> {
        let expanded_path = expand_path(path_input)
            .map_err(|err| AltreError::Application(format!("パス展開エラー: {}", err)))?;

        if let Some(existing_id) = self.find_buffer_id_by_path(&expanded_path) {
            self.load_buffer_by_id(existing_id, true)?;
            return Ok(format!("既存のバッファに切り替えました: {}", expanded_path.display()));
        }

        let mut file_manager = FileOperationManager::new();
        let file_buffer = match file_manager.open_file(expanded_path.clone()) {
            Ok(buffer) => buffer,
            Err(AltreError::File(FileError::NotFound { .. })) => file_manager
                .create_new_file_buffer(expanded_path.clone())
                .map_err(|err| AltreError::Application(format!("ファイル操作エラー: {}", err)))?,
            Err(err) => return Err(err),
        };

        let id = self.allocate_buffer_id();
        self.buffers.push(OpenBuffer::new(id, file_buffer));

        self.load_buffer_by_id(id, true)?;

        Ok(format!("ファイルを開きました: {}", expanded_path.display()))
    }

    fn current_viewport_mut(&mut self) -> &mut ViewportState {
        self
            .window_manager
            .focused_viewport_mut()
            .expect("フォーカスウィンドウが存在しません")
    }

    fn current_viewport(&self) -> &ViewportState {
        self
            .window_manager
            .focused_viewport()
            .expect("フォーカスウィンドウが存在しません")
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

        if self.replace.controller.is_active() {
            if self.handle_replace_key(key_event)? {
                return Ok(());
            }
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

    fn handle_replace_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        use KeyCode::*;
        use KeyModifiers as KM;

        if !self.replace.controller.is_active() {
            return Ok(false);
        }

        match (key_event.code, key_event.modifiers) {
            (Char('y'), KM::NONE) | (Char('Y'), KM::NONE) | (Char(' '), KM::NONE) => {
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.begin_history(HistoryCommandKind::Other);
                match self.replace.controller.accept_current(&mut self.editor) {
                    Ok(progress) => {
                        self.end_history(true);
                        self.after_replace_progress(progress);
                    }
                    Err(err) => {
                        self.end_history(false);
                        self.show_error_message(err);
                    }
                }
                Ok(true)
            }
            (Char('n'), KM::NONE) | (Char('N'), KM::NONE) => {
                let progress = self.replace.controller.skip_current();
                self.after_replace_progress(progress);
                Ok(true)
            }
            (Delete, _) | (Backspace, _) => {
                let progress = self.replace.controller.skip_current();
                self.after_replace_progress(progress);
                Ok(true)
            }
            (Char('!'), KM::NONE) => {
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.begin_history(HistoryCommandKind::Other);
                match self.replace.controller.accept_all(&mut self.editor) {
                    Ok(progress) => {
                        self.end_history(true);
                        self.after_replace_progress(progress);
                    }
                    Err(err) => {
                        self.end_history(false);
                        self.show_error_message(err);
                    }
                }
                Ok(true)
            }
            (Enter, KM::NONE) | (Char('q'), KM::NONE) | (Char('Q'), KM::NONE) => {
                let summary = self.replace.controller.finish();
                self.finish_replace_session(summary);
                Ok(true)
            }
            (Char('g'), modifiers) if modifiers.contains(KM::CONTROL) => {
                self.begin_history(HistoryCommandKind::Other);
                match self.replace.controller.cancel(&mut self.editor) {
                    Ok(summary) => {
                        self.end_history(true);
                        self.finish_replace_session(summary);
                    }
                    Err(err) => {
                        self.end_history(false);
                        self.show_error_message(err);
                    }
                }
                Ok(true)
            }
            _ => {
                self.show_info_message("yで置換、nでスキップ、!で残りを置換、qで終了、C-gでキャンセル");
                Ok(true)
            }
        }
    }

    fn start_query_replace_session(&mut self, pattern: String, replacement: String, use_regex: bool) -> Result<()> {
        if pattern.is_empty() {
            self.show_error_message(AltreError::Application("置換パターンが空です".to_string()));
            return Ok(());
        }

        if self.replace.controller.is_active() {
            let summary = self.replace.controller.finish();
            self.finish_replace_session(summary);
        }

        if self.search.is_active() {
            self.search.cancel(&mut self.editor);
        }

        let case_sensitive = pattern.chars().any(|c| c.is_uppercase());
        let snapshot = self.editor.to_string();

        let start = if use_regex {
            match self
                .replace
                .controller
                .start_regex(&snapshot, pattern.clone(), replacement.clone(), case_sensitive)
            {
                Ok(info) => info,
                Err(err) => {
                    self.show_error_message(AltreError::Application(format!(
                        "正規表現エラー: {}",
                        err
                    )));
                    self.replace.reset();
                    return Ok(());
                }
            }
        } else {
            self.replace
                .controller
                .start_literal(&snapshot, pattern.clone(), replacement.clone(), case_sensitive)
        };

        if start.total_matches == 0 || !self.replace.controller.is_active() {
            self.replace.highlights.clear();
            self.show_info_message("置換対象が見つかりません");
            return Ok(());
        }

        if let Some((start_pos, _)) = self.replace.controller.current_range() {
            let _ = self.editor.move_cursor_to_char(start_pos);
        }

        self.reset_kill_context();
        self.reset_recenter_cycle();
        self.update_replace_view();
        Ok(())
    }

    fn after_replace_progress(&mut self, progress: ReplaceProgress) {
        if !self.replace.controller.is_active() || progress.finished {
            let summary = self.replace.controller.finish();
            self.finish_replace_session(summary);
        } else {
            self.update_replace_view();
        }
    }

    fn update_replace_view(&mut self) {
        if !self.replace.controller.is_active() {
            self.replace.highlights.clear();
            self.minibuffer.set_status_message(None);
            return;
        }

        let snapshot = self.editor.to_string();
        self.replace.highlights = self.replace.controller.highlights(&snapshot);
        if let Some((start_pos, _)) = self.replace.controller.current_range() {
            let _ = self.editor.move_cursor_to_char(start_pos);
        }
        self.minibuffer
            .set_status_message(self.replace_prompt_message(&snapshot));
        self.ensure_cursor_visible();
    }

    fn replace_prompt_message(&self, snapshot: &str) -> Option<String> {
        let (original, replacement, index, total) = self.replace.controller.current_preview(snapshot)?;
        let label = if self.replace.controller.is_regex() {
            "Regex replace"
        } else {
            "Query replace"
        };
        Some(format!(
            "{} {}/{}: \"{}\" → \"{}\" (y=Yes, n=No, !=All, q=Quit)",
            label,
            index,
            total,
            Self::preview_fragment(&original),
            Self::preview_fragment(&replacement)
        ))
    }

    fn finish_replace_session(&mut self, summary: ReplaceSummary) {
        self.replace.reset();
        self.minibuffer.set_status_message(None);
        if summary.cancelled {
            self.show_info_message(format!(
                "置換をキャンセルしました（置換 {} 件、スキップ {} 件）",
                summary.replaced, summary.skipped
            ));
        } else {
            self.show_info_message(format!(
                "置換完了: {} 件置換、{} 件スキップ",
                summary.replaced, summary.skipped
            ));
        }
        self.ensure_cursor_visible();
    }

    fn preview_fragment(input: &str) -> String {
        const MAX_LEN: usize = 24;

        if input.is_empty() {
            return "<empty>".to_string();
        }

        let mut result = String::new();
        for (idx, ch) in input.chars().enumerate() {
            if idx >= MAX_LEN {
                result.push('…');
                break;
            }
            let display = match ch {
                '\n' => '↵',
                '\t' => '⇥',
                _ => ch,
            };
            result.push(display);
        }
        result
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
            Command::ForwardWord => {
                self.navigate(NavigationAction::MoveWordForward);
                Ok(())
            }
            Command::BackwardWord => {
                self.navigate(NavigationAction::MoveWordBackward);
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
                self.begin_history(HistoryCommandKind::InsertChar);
                let result = self.editor.insert_char(ch);
                let success = result.is_ok();
                if let Err(err) = result {
                    self.show_error_message(err);
                }
                self.end_history(success);
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::DeleteBackwardChar => {
                self.begin_history(HistoryCommandKind::DeleteBackward);
                let result = self.editor.delete_backward();
                let success = result.is_ok();
                if let Err(err) = result {
                    self.show_error_message(err);
                }
                self.end_history(success);
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::DeleteChar => {
                self.begin_history(HistoryCommandKind::Other);
                let result = self.editor.delete_forward();
                let success = result.is_ok();
                if let Err(err) = result {
                    self.show_error_message(err);
                }
                self.end_history(success);
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::InsertNewline => {
                self.begin_history(HistoryCommandKind::Other);
                let result = self.editor.insert_newline();
                let success = result.is_ok();
                if let Err(err) = result {
                    self.show_error_message(err);
                }
                self.end_history(success);
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                Ok(())
            }
            Command::IndentForTab => {
                self.indent_for_tab();
                Ok(())
            }
            Command::NewlineAndIndent => {
                self.newline_and_indent();
                Ok(())
            }
            Command::OpenLine => {
                self.open_line();
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
            Command::Undo => {
                if let Err(err) = self.undo_edit() {
                    self.show_error_message(err);
                }
                Ok(())
            }
            Command::Redo => {
                if let Err(err) = self.redo_edit() {
                    self.show_error_message(err);
                }
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
            Command::SplitWindowBelow => {
                self.split_window(SplitOrientation::Horizontal);
                Ok(())
            }
            Command::SplitWindowRight => {
                self.split_window(SplitOrientation::Vertical);
                Ok(())
            }
            Command::DeleteOtherWindows => {
                self.delete_other_windows();
                Ok(())
            }
            Command::DeleteWindow => {
                self.delete_current_window();
                Ok(())
            }
            Command::OtherWindow => {
                self.focus_next_window();
                Ok(())
            }
            Command::SwitchToBuffer => {
                let buffers = self.buffer_names();
                let initial = self.last_buffer_name();
                self.minibuffer.start_switch_buffer(&buffers, initial.as_deref())?;
                Ok(())
            }
            Command::KillBuffer => {
                let buffers = self.buffer_names();
                let current_name = self.current_buffer().map(|buffer| buffer.name().to_string());
                self.minibuffer.start_kill_buffer(&buffers, current_name.as_deref())?;
                Ok(())
            }
            Command::ListBuffers => {
                self.show_buffer_list();
                Ok(())
            }
            Command::WriteFile => {
                // C-x C-w 実行時は常にファイルパスを確認
                if let Some(buffer) = self.current_buffer() {
                    let suggested = if let Some(ref path) = buffer.file.path {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("untitled")
                            .to_string()
                    } else if buffer.file.name.trim().is_empty() {
                        "untitled".to_string()
                    } else {
                        buffer.file.name.clone()
                    };
                    self.start_save_as_prompt(&suggested)?;
                } else {
                    self.start_save_as_prompt("untitled")?;
                }
                Ok(())
            }

            Command::SaveAllBuffers => {
                self.persist_current_buffer_state();

                let original_id = self.current_buffer_id;
                let mut saved_count = 0usize;

                for idx in 0..self.buffers.len() {
                    let buffer_clone = self.buffers[idx].file.clone();
                    self.command_processor.set_current_buffer(buffer_clone);
                    let content = self.buffers[idx].file.content.clone();
                    self.command_processor.sync_editor_content(&content);

                    let result = self.command_processor.execute(Command::SaveBuffer);
                    if result.success {
                        if let Some(updated) = self.command_processor.current_buffer().cloned() {
                            self.buffers[idx].file = updated;
                        }
                        saved_count += 1;
                    } else if let Some(msg) = result.message {
                        self.show_error_message(AltreError::Application(msg));
                        return Ok(());
                    }
                }

                if let Some(id) = original_id {
                    if let Some(index) = self.find_buffer_index(id) {
                        let buffer_clone = self.buffers[index].file.clone();
                        self.command_processor.set_current_buffer(buffer_clone);
                        self.command_processor.sync_editor_content(&self.editor.to_string());
                    }
                }

                self.show_info_message(format!("{} 個のバッファを保存しました", saved_count));
                Ok(())
            }

            Command::SaveBuffer => {
                self.persist_current_buffer_state();

                if let Some(index) = self.current_buffer_index() {
                    let needs_path = self.buffers[index].file.path.is_none();
                    if needs_path {
                        let suggested = if self.buffers[index].file.name.trim().is_empty() {
                            "untitled".to_string()
                        } else {
                            self.buffers[index].file.name.clone()
                        };
                        self.start_save_as_prompt(&suggested)?;
                    } else {
                        let buffer_clone = self.buffers[index].file.clone();
                        self.command_processor.set_current_buffer(buffer_clone);
                        self.command_processor.sync_editor_content(&self.editor.to_string());

                        let result = self.command_processor.execute(Command::SaveBuffer);
                        if result.success {
                            if let Some(updated) = self.command_processor.current_buffer().cloned() {
                                self.buffers[index].file = updated;
                            }
                            if let Some(msg) = result.message {
                                self.show_info_message(msg);
                            }
                        } else if let Some(msg) = result.message {
                            self.show_error_message(AltreError::Application(msg));
                        }
                    }
                } else {
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
                Ok(())
            }
            Command::SaveBuffersKillTerminal | Command::Quit => {
                self.persist_current_buffer_state();
                self.shutdown();
                Ok(())
            }
            Command::ExecuteCommand => {
                self.start_execute_command_prompt()
            }
            Command::EvalExpression => {
                self.start_eval_expression_prompt()
            }
            Command::QueryReplace => {
                self.start_query_replace_prompt(false)
            }
            Command::RegexQueryReplace => {
                self.start_query_replace_prompt(true)
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

    fn indent_for_tab(&mut self) {
        self.begin_history(HistoryCommandKind::Other);
        let insertion = self.tab_insertion_string();
        let result = self.editor.insert_str(&insertion);
        let success = result.is_ok();

        if let Err(err) = result {
            self.show_error_message(err);
        }

        self.end_history(success);
        self.reset_kill_context();
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
    }

    fn newline_and_indent(&mut self) {
        self.begin_history(HistoryCommandKind::Other);
        let indent = self.current_line_indent();
        let mut success = false;

        match self.editor.insert_newline() {
            Ok(()) => {
                let result = if indent.is_empty() {
                    Ok(())
                } else {
                    self.editor.insert_str(&indent)
                };

                if let Err(err) = result {
                    self.show_error_message(err);
                } else {
                    success = true;
                }
            }
            Err(err) => {
                self.show_error_message(err);
            }
        }

        self.end_history(success);
        self.reset_kill_context();
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
    }

    fn open_line(&mut self) {
        self.begin_history(HistoryCommandKind::Other);
        let cursor_before = *self.editor.cursor();
        let mut success = false;

        match self.editor.insert_newline() {
            Ok(()) => {
                self.editor.set_cursor(cursor_before);
                success = true;
            }
            Err(err) => {
                self.show_error_message(err);
            }
        }

        self.end_history(success);
        self.reset_kill_context();
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
    }

    fn current_line_indent(&self) -> String {
        let cursor = *self.editor.cursor();
        let text = self.editor.to_string();
        let lines: Vec<&str> = text.split('\n').collect();

        let line_content = if cursor.line < lines.len() {
            lines[cursor.line]
        } else {
            lines.last().copied().unwrap_or("")
        };

        line_content
            .chars()
            .take_while(|ch| matches!(ch, ' ' | '\t'))
            .collect()
    }

    fn tab_insertion_string(&self) -> String {
        let cursor = *self.editor.cursor();
        let text = self.editor.to_string();
        let line_content = text.split('\n').nth(cursor.line).unwrap_or("");
        let spaces = edit_utils::spaces_to_next_tab_stop(line_content, cursor.column, DEFAULT_TAB_WIDTH);
        " ".repeat(spaces)
    }

    fn kill_word_forward(&mut self) {
        self.begin_history(HistoryCommandKind::Other);
        let result = self.editor.delete_word_forward();
        let success = result.is_ok();
        match result {
            Ok(text) => {
                self.record_kill(text, KillMerge::Append);
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => self.show_error_message(err),
        }
        self.end_history(success);
    }

    fn kill_word_backward(&mut self) {
        self.begin_history(HistoryCommandKind::Other);
        let result = self.editor.delete_word_backward();
        let success = result.is_ok();
        match result {
            Ok(text) => {
                self.record_kill(text, KillMerge::Prepend);
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => self.show_error_message(err),
        }
        self.end_history(success);
    }

    fn kill_line_forward(&mut self) {
        self.begin_history(HistoryCommandKind::Other);
        let result = self.editor.kill_line_forward();
        let success = result.is_ok();
        match result {
            Ok(text) => {
                self.record_kill(text, KillMerge::Append);
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
            }
            Err(err) => self.show_error_message(err),
        }
        self.end_history(success);
    }

    fn set_mark_command(&mut self) {
        self.editor.set_mark();
        self.show_info_message("マークを設定しました");
        self.reset_recenter_cycle();
    }

    fn kill_region(&mut self) -> Result<()> {
        self.begin_history(HistoryCommandKind::Other);
        let result = if let Some((start, end)) = self.editor.selection_range() {
            match self.editor.delete_range_span(start, end) {
                Ok(text) => {
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
                    Ok(())
                }
                Err(err) => {
                    self.show_error_message(err.clone());
                    Err(err)
                }
            }
        } else {
            self.show_info_message("リージョンが選択されていません");
            Ok(())
        };
        self.end_history(result.is_ok());
        result
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

        self.begin_history(HistoryCommandKind::Other);
        match self.editor.insert_str(&text) {
            Ok(_) => {
                self.kill_context = KillContext::Yank;
                self.last_yank_range = Some((start, len));
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                self.end_history(true);
            }
            Err(err) => {
                self.reset_kill_context();
                self.show_error_message(err);
                self.end_history(false);
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

        self.begin_history(HistoryCommandKind::Other);
        if let Err(err) = self.editor.delete_range(start, start + previous_len) {
            self.reset_kill_context();
            self.show_error_message(err);
            self.end_history(false);
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
            self.end_history(false);
            return;
        }

        self.kill_context = KillContext::Yank;
        self.last_yank_range = Some((start, new_len));
        self.reset_recenter_cycle();
        self.ensure_cursor_visible();
        self.end_history(true);
    }

    fn undo_edit(&mut self) -> Result<()> {
        match self.history.undo(&mut self.editor) {
            Ok(true) => {
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                self.persist_current_buffer_state();
                Ok(())
            }
            Ok(false) => {
                self.show_info_message("取り消す操作はありません");
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn redo_edit(&mut self) -> Result<()> {
        match self.history.redo(&mut self.editor) {
            Ok(true) => {
                self.reset_kill_context();
                self.reset_recenter_cycle();
                self.ensure_cursor_visible();
                self.persist_current_buffer_state();
                Ok(())
            }
            Ok(false) => {
                self.show_info_message("やり直す操作はありません");
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn keyboard_quit(&mut self) {
        self.reset_kill_context();
        self.reset_recenter_cycle();
        let mut replaced_session = false;
        if self.replace.controller.is_active() {
            self.begin_history(HistoryCommandKind::Other);
            match self.replace.controller.cancel(&mut self.editor) {
                Ok(summary) => {
                    self.end_history(true);
                    self.finish_replace_session(summary);
                    replaced_session = true;
                }
                Err(err) => {
                    self.end_history(false);
                    self.show_error_message(err);
                }
            }
        }
        if self.search.is_active() {
            self.search.cancel(&mut self.editor);
        }
        self.editor.clear_mark();
        if !replaced_session {
            self.show_info_message("キャンセルしました");
        }
        self.ensure_cursor_visible();
    }

    fn reset_kill_context(&mut self) {
        self.kill_context = KillContext::None;
        self.last_yank_range = None;
    }

    fn begin_history(&mut self, kind: HistoryCommandKind) {
        self.history.begin_command(kind, &self.editor);
    }

    fn end_history(&mut self, success: bool) {
        self.history.end_command(&self.editor, success);
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
                    kind: HighlightKind::Selection,
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
        let cursor_line = self.editor.cursor().line;
        let cursor_column = self.editor.cursor().column;

        {
            let viewport = self.current_viewport_mut();
            viewport.clamp_vertical(total_lines);

            let height = viewport.height.max(1);
            if cursor_line < viewport.top_line {
                viewport.top_line = cursor_line;
            } else if cursor_line >= viewport.top_line + height {
                viewport.top_line = cursor_line + 1 - height;
            }

            viewport.clamp_vertical(total_lines);

            if cursor_column < viewport.scroll_x {
                viewport.scroll_x = cursor_column;
            } else if cursor_column >= viewport.scroll_x + viewport.width {
                viewport.scroll_x = cursor_column + 1 - viewport.width;
            }

            viewport.clamp_horizontal(max_columns);
        }
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
        let height = self.current_viewport().height.max(1);
        let step = height.saturating_sub(1).max(1);
        let old_top = self.current_viewport().top_line;
        let max_top = total_lines.saturating_sub(height);
        let new_top = (old_top + step).min(max_top);
        let delta = new_top.saturating_sub(old_top);
        {
            let viewport = self.current_viewport_mut();
            viewport.top_line = new_top;
        }
        if delta > 0 {
            self.move_cursor_vertical(delta as isize);
        }
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn scroll_page_up(&mut self) {
        let height = self.current_viewport().height.max(1);
        let step = height.saturating_sub(1).max(1);
        let old_top = self.current_viewport().top_line;
        let new_top = old_top.saturating_sub(step);
        let delta = old_top.saturating_sub(new_top);
        {
            let viewport = self.current_viewport_mut();
            viewport.top_line = new_top;
        }
        if delta > 0 {
            self.move_cursor_vertical(-(delta as isize));
        }
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn recenter_view(&mut self) {
        let (total_lines, _) = self.buffer_metrics();
        let height = self.current_viewport().height.max(1);
        let cursor_line = self.editor.cursor().line;
        let max_top = total_lines.saturating_sub(height);

        let desired_top = match self.recenter_step % 3 {
            0 => cursor_line.saturating_sub(height / 2),
            1 => cursor_line,
            _ => cursor_line.saturating_add(1).saturating_sub(height),
        };

        {
            let viewport = self.current_viewport_mut();
            viewport.top_line = desired_top.min(max_top);
        }
        self.recenter_step = (self.recenter_step + 1) % 3;
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn horizontal_scroll_step(&self) -> usize {
        (self.current_viewport().width / 2).max(1)
    }

    fn scroll_left(&mut self) {
        let step = self.horizontal_scroll_step();
        {
            let viewport = self.current_viewport_mut();
            viewport.scroll_x = viewport.scroll_x.saturating_add(step);
        }
        self.move_cursor_horizontal(step as isize);
        self.reset_recenter_cycle();
        self.reset_kill_context();
        self.ensure_cursor_visible();
    }

    fn scroll_right(&mut self) {
        let step = self.horizontal_scroll_step();
        let current_scroll = self.current_viewport().scroll_x;
        if current_scroll > 0 {
            let delta = current_scroll.min(step);
            {
                let viewport = self.current_viewport_mut();
                viewport.scroll_x -= delta;
            }
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

    fn start_query_replace_prompt(&mut self, is_regex: bool) -> Result<()> {
        let mut initial_pattern: Option<String> = None;

        if self.search.is_active() {
            if let Some(pattern) = self.search.current_pattern() {
                if !pattern.is_empty() {
                    initial_pattern = Some(pattern.to_string());
                }
            }
            self.search.accept();
        } else if let Some(pattern) = self.search.last_pattern() {
            if !pattern.is_empty() {
                initial_pattern = Some(pattern.to_string());
            }
        }

        if initial_pattern.is_none() {
            if let Ok(Some(selection)) = self.editor.selection_text() {
                if !selection.is_empty() {
                    initial_pattern = Some(selection);
                }
            }
        }

        let action = MinibufferAction::QueryReplace {
            is_regex,
            initial: initial_pattern,
        };

        match self.minibuffer.handle_event(SystemEvent::Action(action)) {
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
                        debug_log!(self, "Opening file via minibuffer: {}", path);
                        match self.open_file_at_path(&path) {
                            Ok(message) => self.show_info_message(message),
                            Err(err) => self.show_error_message(err),
                        }
                    }
                    FileOperation::SaveAs(path) => {
                        self.persist_current_buffer_state();
                        if let Some(index) = self.current_buffer_index() {
                            if let Some(current) = self.buffers.get(index) {
                                self.command_processor.set_current_buffer(
                                    current.file.clone(),
                                );
                            }
                            self.command_processor.sync_editor_content(&self.editor.to_string());
                            let result = self.command_processor.save_buffer_as(path.clone());
                            if result.success {
                                if let Some(updated) = self.command_processor.current_buffer().cloned() {
                                    if let Some(buffer) = self.buffers.get_mut(index) {
                                        buffer.file = updated;
                                        buffer.cursor = *self.editor.cursor();
                                        buffer.history = self.history.stack().clone();
                                    }
                                }
                                if let Some(msg) = result.message {
                                    self.show_info_message(msg);
                                }
                                self.ensure_cursor_visible();
                            } else if let Some(msg) = result.message {
                                self.show_error_message(AltreError::Application(msg));
                            }
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
            Ok(SystemResponse::SwitchBuffer(name)) => {
                let target = if name.trim().is_empty() {
                    self.last_buffer_name()
                } else {
                    Some(name)
                };

                if let Some(buffer_name) = target {
                    if let Err(err) = self.switch_to_buffer_by_name(&buffer_name) {
                        self.show_error_message(err);
                    }
                } else {
                    self.show_error_message(AltreError::Application(
                        "切り替えるバッファが見つかりません".to_string()
                    ));
                }
                Ok(())
            }
            Ok(SystemResponse::KillBuffer(name)) => {
                let trimmed = name.trim();
                let target = if trimmed.is_empty() { None } else { Some(trimmed) };
                if let Err(err) = self.kill_buffer_by_name(target) {
                    self.show_error_message(err);
                }
                Ok(())
            }
            Ok(SystemResponse::ListBuffers) => {
                self.show_buffer_list();
                Ok(())
            }
            Ok(SystemResponse::QueryReplace { pattern, replacement, is_regex }) => {
                self.start_query_replace_session(pattern, replacement, is_regex)?;
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

    fn split_window(&mut self, orientation: SplitOrientation) {
        self.window_manager.split_focused(orientation);
        self.ensure_cursor_visible();
    }

    fn delete_other_windows(&mut self) {
        match self.window_manager.delete_others() {
            Ok(()) => {
                self.ensure_cursor_visible();
            }
            Err(err) => {
                self.show_error_message(AltreError::Application(err.to_string()));
            }
        }
    }

    fn delete_current_window(&mut self) {
        match self.window_manager.delete_focused() {
            Ok(()) => {
                self.ensure_cursor_visible();
            }
            Err(err) => {
                self.show_error_message(AltreError::Application(err.to_string()));
            }
        }
    }

    fn focus_next_window(&mut self) {
        self.window_manager.focus_next();
        self.ensure_cursor_visible();
    }

    fn render<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let search_ui = self.search.ui_state();
        let search_highlights = self.search.highlights();
        let replace_highlights = &self.replace.highlights;
        let selection_highlights = self.selection_highlights();
        let mut combined_highlights = Vec::with_capacity(
            search_highlights.len() + replace_highlights.len() + selection_highlights.len(),
        );
        combined_highlights.extend_from_slice(search_highlights);
        combined_highlights.extend_from_slice(replace_highlights);
        combined_highlights.extend(selection_highlights.into_iter());

        let (file_label_buf, is_modified) = if let Some(buffer) = self.current_buffer() {
            let label = if let Some(path) = buffer.path() {
                path.display().to_string()
            } else if buffer.name().trim().is_empty() {
                "[未保存] *scratch*".to_string()
            } else {
                format!("[未保存] {}", buffer.name())
            };
            (label, buffer.is_modified())
        } else {
            ("[バッファなし]".to_string(), false)
        };

        let status_info = StatusLineInfo {
            file_label: file_label_buf.as_str(),
            is_modified,
        };

        self.renderer
            .render(
                terminal,
                &self.editor,
                &mut self.window_manager,
                &self.minibuffer,
                search_ui,
                &combined_highlights,
                status_info,
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
        let viewport = app
            .window_manager
            .focused_viewport()
            .expect("focused viewport");
        assert_eq!(viewport.top_line, 0);
        assert_eq!(viewport.scroll_x, 0);
    }
}
