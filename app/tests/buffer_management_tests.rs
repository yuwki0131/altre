use altre::app::App;
use altre::input::keybinding::{Action, Key, KeyCode, KeyModifiers, KeyProcessResult, ModernKeyMap};
use std::fs;
use tempfile::tempdir;

fn plain_char(ch: char) -> Key {
    Key {
        modifiers: KeyModifiers { ctrl: false, alt: false, shift: false },
        code: KeyCode::Char(ch),
    }
}

#[test]
fn test_switch_buffer_via_api() {
    let mut app = App::new().expect("app init");
    let scratch_name = app
        .current_buffer_name()
        .expect("default buffer");

    app.insert_str("scratch data").expect("insert");

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("sample.txt");
    fs::write(&file_path, "file buffer").unwrap();

    app.open_file(file_path.to_str().unwrap()).expect("open file");

    let buffer_names = app.buffer_names();
    assert_eq!(buffer_names.len(), 2);
    assert!(buffer_names.iter().any(|name| name == "sample.txt"));

    app.switch_buffer("sample.txt").expect("switch");
    assert_eq!(app.get_buffer_content(), "file buffer");

    app.switch_buffer(&scratch_name).expect("switch back");
    assert_eq!(app.get_buffer_content(), "scratch data");
}

#[test]
fn test_kill_buffer_removes_entry() {
    let mut app = App::new().expect("app init");

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("remove.txt");
    fs::write(&file_path, "temporary").unwrap();

    app.open_file(file_path.to_str().unwrap()).expect("open file");
    assert_eq!(app.buffer_names().len(), 2);

    app.kill_buffer(Some("remove.txt")).expect("kill buffer");
    assert_eq!(app.buffer_names().len(), 1);
    assert_eq!(app.current_buffer_name().unwrap(), "*scratch*".to_string());

    // 最後のバッファは削除できない
    assert!(app.kill_buffer(None).is_ok());
    assert_eq!(app.buffer_names().len(), 1);
}

#[test]
fn test_modern_keymap_buffer_actions() {
    let mut keymap = ModernKeyMap::new();

    assert_eq!(keymap.process_key(Key::ctrl_x()), KeyProcessResult::PartialMatch);
    assert_eq!(
        keymap.process_key(plain_char('b')),
        KeyProcessResult::Action(Action::SwitchBuffer)
    );

    assert_eq!(keymap.process_key(Key::ctrl_x()), KeyProcessResult::PartialMatch);
    assert_eq!(
        keymap.process_key(plain_char('k')),
        KeyProcessResult::Action(Action::KillBuffer)
    );

    assert_eq!(keymap.process_key(Key::ctrl_x()), KeyProcessResult::PartialMatch);
    assert_eq!(
        keymap.process_key(Key::ctrl_b()),
        KeyProcessResult::Action(Action::ListBuffers)
    );
}
