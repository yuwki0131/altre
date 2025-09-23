//! alisp v0 interpreter implementation

mod ast;
pub mod error;
mod evaluator;
mod primitives;
pub mod reader;
mod symbol;
pub mod integration;
mod runtime;

pub use evaluator::{EvalOutcome, Interpreter};
pub use symbol::{SymbolId, SymbolInterner};
pub use error::{EvalError, ReaderError};
