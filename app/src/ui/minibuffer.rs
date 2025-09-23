use crate::alisp::integration::{eval_in_minibuffer, MinibufferOutcome};
use crate::alisp::Interpreter;

pub struct MinibufferSession {
    interpreter: Interpreter,
}

impl MinibufferSession {
    pub fn new() -> Self {
        Self { interpreter: Interpreter::new() }
    }

    pub fn evaluate(&mut self, input: &str) -> MinibufferOutcome {
        eval_in_minibuffer(&mut self.interpreter, input)
    }
}
