use crate::core::{Backend, RenderMetadata, RenderView};
use crate::error::{AltreError, Result, UiError};
use crate::ui::{AdvancedRenderer, StatusLineInfo};
use crossterm::event::{self, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::time::Duration;

pub struct TuiApplication {
    backend: Backend,
    renderer: AdvancedRenderer,
}

impl TuiApplication {
    pub fn new() -> Result<Self> {
        let backend = Backend::new()?;
        let renderer = AdvancedRenderer::new();
        Ok(Self { backend, renderer })
    }

    pub fn run(&mut self) -> Result<()> {
        enter_terminal()?;

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend).map_err(|err| terminal_error("terminal init", err))?;
        terminal.hide_cursor().map_err(|err| terminal_error("hide cursor", err))?;

        let loop_result = self.event_loop(&mut terminal);
        let show_cursor_result = terminal.show_cursor().map_err(|err| terminal_error("show cursor", err));
        drop(terminal);
        let cleanup_result = leave_terminal();

        loop_result.and(show_cursor_result).and(cleanup_result)
    }

    fn event_loop<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while self.backend.is_running() {
            self.backend.process_minibuffer_timer();
            self.render(terminal)?;

            if event::poll(Duration::from_millis(16)).map_err(|err| terminal_error("event poll", err))? {
                match event::read().map_err(|err| terminal_error("event read", err))? {
                    Event::Key(key_event) => self.backend.handle_key_event(key_event)?,
                    Event::Resize(_, _) => {}
                    Event::Mouse(_) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
                }
            }
        }

        Ok(())
    }

    fn render<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let metadata: RenderMetadata = self.backend.render_metadata();
        let view: RenderView<'_> = self.backend.render_view();

        let status_info = StatusLineInfo {
            file_label: metadata.status_label.as_str(),
            is_modified: metadata.is_modified,
        };

        self.renderer
            .render(
                terminal,
                view.editor,
                view.window_manager,
                view.minibuffer,
                metadata.search_ui.as_ref(),
                &metadata.highlights,
                status_info,
            )
            .map_err(|err| terminal_error("render", err))
    }
}

fn enter_terminal() -> Result<()> {
    enable_raw_mode().map_err(|err| terminal_error("enable raw mode", err))?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen).map_err(|err| terminal_error("enter alternate screen", err))?;
    Ok(())
}

fn leave_terminal() -> Result<()> {
    let mut out = stdout();
    execute!(out, LeaveAlternateScreen).map_err(|err| terminal_error("leave alternate screen", err))?;
    disable_raw_mode().map_err(|err| terminal_error("disable raw mode", err))?;
    Ok(())
}

fn terminal_error(context: &str, err: impl std::fmt::Display) -> AltreError {
    AltreError::Ui(UiError::RenderingFailed {
        component: format!("{}: {}", context, err),
    })
}
