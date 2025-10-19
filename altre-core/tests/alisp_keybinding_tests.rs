use altre::buffer::navigation::NavigationAction;
use altre::input::keybinding::Action;
use altre::App;
use std::fs;
use std::path::PathBuf;

fn setup_temp_home(script: &str) -> (tempfile::TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let home = temp_dir.path().join("home");
    let config_dir = home.join(".altre");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("init.al"), script).expect("write init.al");
    (temp_dir, home)
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(ref value) = self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

#[test]
fn user_init_alisp_overrides_default_keybindings() {
    let script = r#"(bind-key "C-x C-f" "goto-line")"#;
    let (_tmp, home) = setup_temp_home(script);
    let _home_guard = EnvVarGuard::set("HOME", home.to_string_lossy().as_ref());

    let app = App::new().expect("initialize app");
    let keymap = app.keymap_handle();
    let keymap_ref = keymap.borrow();

    assert_eq!(
        keymap_ref.lookup_action("C-n"),
        Some(Action::Navigate(NavigationAction::MoveLineDown))
    );
    assert_eq!(keymap_ref.lookup_action("C-x C-f"), Some(Action::GotoLine));
}
