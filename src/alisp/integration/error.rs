use crate::alisp::error::{EvalError, EvalErrorKind};
use crate::alisp::symbol::SymbolInterner;

pub fn format_eval_error(err: &EvalError, interner: &SymbolInterner) -> String {
    match &err.kind {
        EvalErrorKind::NameNotFound(sym) => {
            let name = interner.resolve(*sym).unwrap_or("<unknown>");
            format!("alisp error: name - 未定義のシンボル {}", name)
        }
        EvalErrorKind::ArityMismatch { expected, found } => {
            format!(
                "alisp error: arity - 引数 {} 個に対して {} 個が渡されました",
                expected, found
            )
        }
        EvalErrorKind::TypeMismatch { expected, found } => {
            format!(
                "alisp error: type - {} が必要ですが {} が渡されました",
                expected, found
            )
        }
        EvalErrorKind::DivisionByZero => "alisp error: runtime - 0 で除算できません".to_string(),
        EvalErrorKind::InvalidLetBinding => {
            "alisp error: syntax - let の束縛が不正です".to_string()
        }
        EvalErrorKind::InvalidDefineTarget => {
            "alisp error: syntax - define の左辺が不正です".to_string()
        }
        EvalErrorKind::Reader(reader) => format!("alisp error: reader - {}", reader.message),
        EvalErrorKind::Runtime(msg) => format!("alisp error: runtime - {}", msg),
    }
}
