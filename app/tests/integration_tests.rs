use altre::{App, Result};
use tempfile::TempDir;
use std::path::Path;

#[test]
fn test_app_initialization() -> Result<()> {
    // Test basic app creation
    let app = App::new()?;
    assert!(app.is_initialized());
    Ok(())
}

#[test]
fn test_file_operations() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Write test content
    std::fs::write(&file_path, "Hello, World!").unwrap();

    // Test file loading
    let mut app = App::new()?;
    app.open_file(file_path.to_str().unwrap())?;

    assert!(app.has_buffer());
    Ok(())
}

#[test]
fn test_basic_editing() -> Result<()> {
    let mut app = App::new()?;

    // Test basic text insertion
    app.insert_char('H')?;
    app.insert_char('i')?;

    assert_eq!(app.get_buffer_content(), "Hi");
    Ok(())
}

#[test]
fn test_cursor_movement() -> Result<()> {
    let mut app = App::new()?;

    // Insert some text
    app.insert_str("Hello\nWorld")?;

    // Test cursor movements
    app.move_cursor_to_start()?;
    assert_eq!(app.get_cursor_position().line, 0);
    assert_eq!(app.get_cursor_position().column, 0);

    Ok(())
}