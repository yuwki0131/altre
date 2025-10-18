#![cfg(feature = "gui")]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::core::{Backend, RenderMetadata};
use crate::error::{AltreError, Result, UiError};
use crate::minibuffer::MinibufferMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use slint::private_unstable_api::re_exports::KeyEvent as SlintKeyEvent;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

mod components;
use components::{AppWindow, CursorData, EditorData, MinibufferData, MinibufferVisual, ModeLineData};

const KEY_ESCAPE: char = '\u{001b}';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_RETURN: char = '\u{000a}';
const KEY_TAB: char = '\u{0009}';
const KEY_DELETE: char = '\u{007f}';
const KEY_INSERT: char = '\u{F727}';
const KEY_HOME: char = '\u{F729}';
const KEY_END: char = '\u{F72B}';
const KEY_PAGE_UP: char = '\u{F72C}';
const KEY_PAGE_DOWN: char = '\u{F72D}';
const KEY_LEFT: char = '\u{F702}';
const KEY_RIGHT: char = '\u{F703}';
const KEY_UP: char = '\u{F700}';
const KEY_DOWN: char = '\u{F701}';
const KEY_F1: char = '\u{F704}';
const KEY_F2: char = '\u{F705}';
const KEY_F3: char = '\u{F706}';
const KEY_F4: char = '\u{F707}';
const KEY_F5: char = '\u{F708}';
const KEY_F6: char = '\u{F709}';
const KEY_F7: char = '\u{F70A}';
const KEY_F8: char = '\u{F70B}';
const KEY_F9: char = '\u{F70C}';
const KEY_F10: char = '\u{F70D}';
const KEY_F11: char = '\u{F70E}';
const KEY_F12: char = '\u{F70F}';

pub struct GuiApplication {
    backend: Rc<RefCell<Backend>>,
    window: AppWindow,
    timer: Timer,
}

impl GuiApplication {
    pub fn new() -> Result<Self> {
        let backend = Rc::new(RefCell::new(Backend::new()?));
        let window = AppWindow::new().map_err(gui_error)?;
        let timer = Timer::default();
        Ok(Self { backend, window, timer })
    }

    pub fn run(&mut self) -> Result<()> {
        self.setup_callbacks();
        self.start_timer();
        self.refresh_view();

        self.window.show().map_err(gui_error)?;
        slint::run_event_loop().map_err(gui_error)?;
        Ok(())
    }

    fn setup_callbacks(&mut self) {
        let backend_rc = self.backend.clone();
        let window_weak = self.window.as_weak();

        self.window.on_key_event(move |event| {
            if let Some(mut backend) = backend_rc.try_borrow_mut().ok() {
                if let Some(key_event) = slint_to_crossterm(&event) {
                    if let Err(err) = backend.handle_key_event(key_event) {
                        eprintln!("GUI key handling error: {}", err);
                    }
                }
                update_view(&window_weak, &mut backend);
            }
        });
    }

    fn start_timer(&mut self) {
        let backend_rc = self.backend.clone();
        let window_weak = self.window.as_weak();

        self.timer.start(TimerMode::Repeated, Duration::from_millis(80), move || {
            if let Some(mut backend) = backend_rc.try_borrow_mut().ok() {
                backend.process_minibuffer_timer();
                let running = backend.is_running();
                update_view(&window_weak, &mut backend);
                if !running {
                    slint::quit_event_loop().ok();
                }
            }
        });
    }

    fn refresh_view(&mut self) {
        if let Some(mut backend) = self.backend.try_borrow_mut().ok() {
            update_view(&self.window.as_weak(), &mut backend);
        }
    }
}

fn update_view(window: &slint::Weak<AppWindow>, backend: &mut Backend) {
    let Some(app) = window.upgrade() else { return; };

    let metadata: RenderMetadata = backend.render_metadata();
    let view = backend.render_view();

    let buffer_text = view.editor.to_string();
    let lines_vec: Vec<SharedString> = buffer_text.lines().map(SharedString::from).collect();
    let total_lines = if lines_vec.is_empty() { 1 } else { lines_vec.len() } as i32;
    let editor_data = EditorData {
        lines: ModelRc::new(VecModel::from(lines_vec)),
        cursor: CursorData {
            line: view.editor.cursor().line as i32,
            column: view.editor.cursor().column as i32,
        },
    };
    app.set_editor(editor_data);

    let mini_state = view.minibuffer.minibuffer_state().clone();
    let completion_vec: Vec<SharedString> =
        mini_state.completions.into_iter().map(SharedString::from).collect();

    let minibuffer_data = MinibufferData {
        style: match mini_state.mode {
            MinibufferMode::Inactive => MinibufferVisual::Inactive,
            MinibufferMode::ErrorDisplay { .. } => MinibufferVisual::Error,
            MinibufferMode::InfoDisplay { .. } => MinibufferVisual::Info,
            _ => MinibufferVisual::Input,
        },
        prompt: SharedString::from(mini_state.prompt),
        input: SharedString::from(mini_state.input),
        message: SharedString::from(mini_state.status_message.unwrap_or_default()),
        completions: ModelRc::new(VecModel::from(completion_vec)),
    };
    app.set_minibuffer(minibuffer_data);

    let mode_line = ModeLineData {
        label: SharedString::from(metadata.status_label.clone()),
        cursor_line: (view.editor.cursor().line + 1) as i32,
        cursor_column: (view.editor.cursor().column + 1) as i32,
        total_lines,
        is_modified: metadata.is_modified,
    };
    app.set_modeline(mode_line);
}

fn slint_to_crossterm(event: &SlintKeyEvent) -> Option<KeyEvent> {
    let mut modifiers = KeyModifiers::empty();
    if event.modifiers.control {
        modifiers |= KeyModifiers::CONTROL;
    }
    if event.modifiers.alt {
        modifiers |= KeyModifiers::ALT;
    }
    if event.modifiers.shift {
        modifiers |= KeyModifiers::SHIFT;
    }

    let text = event.text.as_str();
    let mut chars = text.chars();
    let first = chars.next()?;

    let key_code = match first {
        KEY_ESCAPE => KeyCode::Esc,
        KEY_BACKSPACE => KeyCode::Backspace,
        KEY_RETURN => KeyCode::Enter,
        KEY_TAB => KeyCode::Tab,
        KEY_LEFT => KeyCode::Left,
        KEY_RIGHT => KeyCode::Right,
        KEY_UP => KeyCode::Up,
        KEY_DOWN => KeyCode::Down,
        KEY_HOME => KeyCode::Home,
        KEY_END => KeyCode::End,
        KEY_PAGE_UP => KeyCode::PageUp,
        KEY_PAGE_DOWN => KeyCode::PageDown,
        KEY_INSERT => KeyCode::Insert,
        KEY_DELETE => KeyCode::Delete,
        KEY_F1 => KeyCode::F(1),
        KEY_F2 => KeyCode::F(2),
        KEY_F3 => KeyCode::F(3),
        KEY_F4 => KeyCode::F(4),
        KEY_F5 => KeyCode::F(5),
        KEY_F6 => KeyCode::F(6),
        KEY_F7 => KeyCode::F(7),
        KEY_F8 => KeyCode::F(8),
        KEY_F9 => KeyCode::F(9),
        KEY_F10 => KeyCode::F(10),
        KEY_F11 => KeyCode::F(11),
        KEY_F12 => KeyCode::F(12),
        ch => {
            if ch < ' ' && chars.clone().next().is_none() {
                // Control characters without printable mapping are ignored.
                return None;
            }
            KeyCode::Char(ch)
        }
    };

    Some(KeyEvent::new(key_code, modifiers))
}

fn gui_error(err: impl std::fmt::Display) -> AltreError {
    AltreError::Ui(UiError::RenderingFailed {
        component: format!("GUI: {}", err),
    })
}
