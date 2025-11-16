use altre::buffer::TextEditor;
use altre::minibuffer::{MinibufferAction, MinibufferSystem, SystemEvent};
use altre::ui::layout::LayoutManager;
use altre::ui::renderer::StatusLineInfo;
use altre::ui::window_manager::WindowManager;
use altre::ui::AdvancedRenderer;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

#[test]
fn query_replace_prompt_is_rendered() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut renderer = AdvancedRenderer::new();
    let editor = TextEditor::new();
    let mut windows = WindowManager::new();
    let mut minibuffer = MinibufferSystem::new();

    // start query replace with initial pattern
    minibuffer
        .handle_event(SystemEvent::Action(MinibufferAction::QueryReplace {
            is_regex: false,
            initial: Some("foo".to_string()),
        }))
        .unwrap();

    assert!(
        matches!(
            minibuffer.minibuffer_state().mode,
            altre::minibuffer::MinibufferMode::QueryReplacePattern
        ),
        "unexpected minibuffer mode: {:?}",
        minibuffer.minibuffer_state().mode
    );

    let layout = LayoutManager::new();
    let area_map = layout.calculate_areas(Rect::new(0, 0, 80, 20), minibuffer.is_active(), true);
    let minibuffer_rect = area_map
        .get(&altre::ui::layout::AreaType::Minibuffer)
        .copied()
        .unwrap();

    renderer
        .render(
            &mut terminal,
            &editor,
            &mut windows,
            &minibuffer,
            None,
            &[],
            StatusLineInfo {
                file_label: "test",
                is_modified: false,
            },
        )
        .unwrap();

    let backend = terminal.backend();
    let buffer = backend.buffer();

    // ミニバッファはレイアウトが返した位置に描画される想定
    let width = minibuffer_rect.width as usize;
    let mut minibuffer_line = String::new();
    let start =
        (minibuffer_rect.y as usize) * (buffer.area().width as usize) + minibuffer_rect.x as usize;
    for cell in &buffer.content()[start..start + width] {
        minibuffer_line.push_str(cell.symbol());
    }

    assert!(
        minibuffer_line.contains("Query replace"),
        "minibuffer line missing prompt: {}",
        minibuffer_line
    );
}
