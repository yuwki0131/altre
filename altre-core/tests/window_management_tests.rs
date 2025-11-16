use altre::input::commands::Command;
use altre::input::keybinding::{
    Action, Key, KeyCode, KeyModifiers, KeyProcessResult, ModernKeyMap,
};
use altre::ui::{SplitOrientation, WindowManager};

fn plain_key(ch: char) -> Key {
    Key {
        modifiers: KeyModifiers {
            ctrl: false,
            alt: false,
            shift: false,
        },
        code: KeyCode::Char(ch),
    }
}

#[test]
fn test_cx2_splits_window_action() {
    let mut keymap = ModernKeyMap::new();
    assert_eq!(
        keymap.process_key(Key::ctrl_x()),
        KeyProcessResult::PartialMatch
    );
    let result = keymap.process_key(plain_key('2'));
    assert_eq!(
        result,
        KeyProcessResult::Action(Action::SplitWindowHorizontally)
    );
}

#[test]
fn test_cx3_splits_window_vertically() {
    let mut keymap = ModernKeyMap::new();
    keymap.process_key(Key::ctrl_x());
    let result = keymap.process_key(plain_key('3'));
    assert_eq!(
        result,
        KeyProcessResult::Action(Action::SplitWindowVertically)
    );
}

#[test]
fn test_window_actions_to_command() {
    assert_eq!(
        Action::SplitWindowHorizontally.to_command(),
        Some(Command::SplitWindowBelow)
    );
    assert_eq!(
        Action::SplitWindowVertically.to_command(),
        Some(Command::SplitWindowRight)
    );
    assert_eq!(
        Action::DeleteOtherWindows.to_command(),
        Some(Command::DeleteOtherWindows)
    );
    assert_eq!(
        Action::DeleteWindow.to_command(),
        Some(Command::DeleteWindow)
    );
    assert_eq!(
        Action::FocusOtherWindow.to_command(),
        Some(Command::OtherWindow)
    );
}

#[test]
fn test_window_command_from_string() {
    assert!(matches!(
        Command::from_string("split-window-below"),
        Command::SplitWindowBelow
    ));
    assert!(matches!(
        Command::from_string("split-window-right"),
        Command::SplitWindowRight
    ));
    assert!(matches!(
        Command::from_string("delete-other-windows"),
        Command::DeleteOtherWindows
    ));
    assert!(matches!(
        Command::from_string("delete-window"),
        Command::DeleteWindow
    ));
    assert!(matches!(
        Command::from_string("other-window"),
        Command::OtherWindow
    ));
}

#[test]
fn test_window_manager_split_and_delete() {
    let mut manager = WindowManager::new();
    let original = manager.focused_window();
    let new_id = manager.split_focused(SplitOrientation::Horizontal);
    assert_eq!(manager.window_count(), 2);
    assert_eq!(manager.focused_window(), original);
    assert!(manager.leaf_order().contains(&new_id));

    manager.focus_next();
    assert_eq!(manager.focused_window(), new_id);
    manager.delete_focused().expect("delete new window");
    assert_eq!(manager.window_count(), 1);
    assert_eq!(manager.focused_window(), original);

    let err = manager
        .delete_focused()
        .expect_err("cannot delete last window");
    assert_eq!(err, altre::ui::WindowError::LastWindow);
}

#[test]
fn test_window_manager_delete_others() {
    let mut manager = WindowManager::new();
    manager.split_focused(SplitOrientation::Vertical);
    manager.split_focused(SplitOrientation::Horizontal);
    assert!(manager.window_count() >= 2);
    manager.delete_others().expect("delete others");
    assert_eq!(manager.window_count(), 1);
}
