use crate::alisp::evaluator::{EvalOutcome, Interpreter};
use crate::alisp::integration::error::format_eval_error;

#[derive(Debug, Clone)]
pub struct MinibufferOutcome {
    pub output: String,
    pub messages: Vec<String>,
    pub is_error: bool,
}

pub fn eval_in_minibuffer(interpreter: &mut Interpreter, source: &str) -> MinibufferOutcome {
    match interpreter.eval(source) {
        Ok(EvalOutcome { display, messages, .. }) => MinibufferOutcome { output: format!("=> {}", display), messages, is_error: false },
        Err(err) => {
            let message = format_eval_error(&err, &interpreter.runtime().interner);
            MinibufferOutcome { output: message, messages: Vec::new(), is_error: true }
        }
    }
}
