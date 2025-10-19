//! alisp v0 interpreter implementation

mod ast;
pub mod error;
mod evaluator;
pub mod integration;
mod primitives;
pub mod reader;
mod runtime;
mod symbol;

pub use error::{EvalError, ReaderError};
pub use evaluator::{EvalOutcome, Interpreter};
pub use runtime::HostBridge;
pub use symbol::{SymbolId, SymbolInterner};
