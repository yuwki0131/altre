use altre::alisp::Interpreter;
use altre::ui::minibuffer::MinibufferSession;

#[test]
fn eval_arithmetic() {
    let mut interp = Interpreter::new();
    let result = interp.eval("(+ 10 20 30)").expect("evaluation succeeds");
    assert_eq!(result.display, "60");
}

#[test]
fn define_function_and_call() {
    let mut interp = Interpreter::new();
    interp.eval("(define (add2 x y) (+ x y))").unwrap();
    let result = interp.eval("(add2 4 5)").unwrap();
    assert_eq!(result.display, "9");
}

#[test]
fn let_scoping() {
    let mut interp = Interpreter::new();
    let result = interp.eval("(let ((x 10) (y 5)) (begin (set! x (+ x y)) x))").unwrap();
    assert_eq!(result.display, "15");
}

#[test]
fn lambda_closure_captures_environment() {
    let mut interp = Interpreter::new();
    interp.eval("(define make-adder (lambda (base) (lambda (x) (+ base x))))").unwrap();
    interp.eval("(define add10 (make-adder 10))").unwrap();
    let result = interp.eval("(add10 5)").unwrap();
    assert_eq!(result.display, "15");
}

#[test]
fn print_emits_message() {
    let mut interp = Interpreter::new();
    let outcome = interp.eval("(print \"hello\")").unwrap();
    assert!(outcome.messages.contains(&"hello".to_string()));
}

#[test]
fn boolean_logic() {
    let mut interp = Interpreter::new();
    assert_eq!(interp.eval("(and #t #f)").unwrap().display, "#f");
    assert_eq!(interp.eval("(or #f #t)").unwrap().display, "#t");
}

#[test]
fn error_for_unknown_symbol() {
    let mut interp = Interpreter::new();
    let err = interp.eval("unknown").unwrap_err();
    assert!(format!("{}", err).contains("未定義"));
}

#[test]
fn minibuffer_session_formats_output() {
    let mut session = MinibufferSession::new();
    let outcome = session.evaluate("(+ 1 2)");
    assert_eq!(outcome.output, "=> 3");
    assert!(!outcome.is_error);
}
