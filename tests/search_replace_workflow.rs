use altre::buffer::TextEditor;
use altre::search::QueryReplaceController;

#[test]
fn query_replace_literal_flow() {
    let mut editor = TextEditor::from_str("foo bar foo");
    let mut controller = QueryReplaceController::new();
    let start = controller.start_literal(
        editor.to_string().as_str(),
        "foo".to_string(),
        "baz".to_string(),
        true,
    );
    assert_eq!(start.total_matches, 2);

    let progress = controller.accept_current(&mut editor).unwrap();
    assert_eq!(editor.to_string(), "baz bar foo");
    assert_eq!(progress.replaced, 1);

    let progress = controller.skip_current();
    assert_eq!(progress.skipped, 1);
    assert!(progress.finished);

    let summary = controller.finish();
    assert_eq!(summary.replaced, 1);
    assert_eq!(summary.skipped, 1);
    assert!(!summary.cancelled);
}

#[test]
fn query_replace_regex_captures() {
    let mut editor = TextEditor::from_str("name: John\nname: Alice");
    let mut controller = QueryReplaceController::new();
    let start = controller
        .start_regex(
            editor.to_string().as_str(),
            "name: (\\w+)".to_string(),
            "user: $1".to_string(),
            true,
        )
        .unwrap();
    assert_eq!(start.total_matches, 2);

    controller.accept_current(&mut editor).unwrap();
    controller.accept_current(&mut editor).unwrap();

    assert_eq!(editor.to_string(), "user: John\nuser: Alice");
    let summary = controller.finish();
    assert_eq!(summary.replaced, 2);
}

#[test]
fn query_replace_cancel_restores_text() {
    let mut editor = TextEditor::from_str("abc def abc");
    let mut controller = QueryReplaceController::new();
    controller.start_literal(
        editor.to_string().as_str(),
        "abc".to_string(),
        "X".to_string(),
        true,
    );
    controller.accept_current(&mut editor).unwrap();
    assert_eq!(editor.to_string(), "X def abc");

    let summary = controller.cancel(&mut editor).unwrap();
    assert_eq!(editor.to_string(), "abc def abc");
    assert!(summary.cancelled);
}
