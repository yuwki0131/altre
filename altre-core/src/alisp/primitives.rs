use crate::alisp::error::{EvalError, EvalErrorKind};
use crate::alisp::runtime::EnvHandle;
use crate::alisp::runtime::{define_symbol, value_to_string, Function, RuntimeState, Value};
use crate::alisp::symbol::SymbolId;

/// インタプリタ初期化時に登録した組込み関数のシンボルを保持する。
/// 現在は再利用シナリオが未実装のため未参照だが、将来的に再バインドや
/// メタプログラミングからアクセスする可能性がある。
#[allow(dead_code)]
pub struct PrimitiveRegistry {
    pub add: SymbolId,
    pub sub: SymbolId,
    pub mul: SymbolId,
    pub div: SymbolId,
    pub eq: SymbolId,
    pub lt: SymbolId,
    pub lte: SymbolId,
    pub gt: SymbolId,
    pub gte: SymbolId,
    pub abs: SymbolId,
    pub floor: SymbolId,
    pub ceil: SymbolId,
    pub not: SymbolId,
    pub print: SymbolId,
    pub type_of: SymbolId,
    pub string_append: SymbolId,
    pub string_length: SymbolId,
    pub bind_key: SymbolId,
    pub set_gui_color: SymbolId,
}

impl PrimitiveRegistry {
    pub fn install(runtime: &mut RuntimeState, env: EnvHandle) -> Self {
        macro_rules! register {
            ($name:expr, $func:expr) => {{
                let sym = runtime.intern($name);
                define_symbol(runtime, env, sym, Value::Function(Function::Builtin($func)));
                sym
            }};
        }

        Self {
            add: register!("+", numeric_add),
            sub: register!("-", numeric_sub),
            mul: register!("*", numeric_mul),
            div: register!("/", numeric_div),
            eq: register!("=", numeric_eq),
            lt: register!("<", numeric_lt),
            lte: register!("<=", numeric_lte),
            gt: register!(">", numeric_gt),
            gte: register!(">=", numeric_gte),
            abs: register!("abs", numeric_abs),
            floor: register!("floor", numeric_floor),
            ceil: register!("ceil", numeric_ceil),
            not: register!("not", boolean_not),
            print: register!("print", primitive_print),
            type_of: register!("type-of", primitive_type_of),
            string_append: register!("string-append", primitive_string_append),
            string_length: register!("string-length", primitive_string_length),
            bind_key: register!("bind-key", primitive_bind_key),
            set_gui_color: register!("set-gui-color", primitive_set_gui_color),
        }
    }
}

fn ensure_arity(args: &[Value], expected: usize) -> Result<(), EvalError> {
    if args.len() != expected {
        return Err(EvalError::new(
            EvalErrorKind::ArityMismatch {
                expected,
                found: args.len(),
            },
            None,
            format!("引数の個数が一致しません: {} が必要です", expected),
        ));
    }
    Ok(())
}

fn ensure_min_arity(args: &[Value], min: usize) -> Result<(), EvalError> {
    if args.len() < min {
        return Err(EvalError::new(
            EvalErrorKind::ArityMismatch {
                expected: min,
                found: args.len(),
            },
            None,
            format!("引数は最低 {} 個必要です", min),
        ));
    }
    Ok(())
}

fn expect_number(value: &Value) -> Result<Number, EvalError> {
    match value {
        Value::Integer(i) => Ok(Number::Integer(*i)),
        Value::Float(f) => Ok(Number::Float(*f)),
        _ => Err(EvalError::new(
            EvalErrorKind::TypeMismatch {
                expected: "number",
                found: value.type_name(),
            },
            None,
            "数値が必要です",
        )),
    }
}

fn expect_string<'a>(runtime: &'a RuntimeState, value: &Value) -> Result<&'a str, EvalError> {
    if let Value::String(handle) = value {
        Ok(runtime.heap.string_ref(*handle))
    } else {
        Err(EvalError::new(
            EvalErrorKind::TypeMismatch {
                expected: "string",
                found: value.type_name(),
            },
            None,
            "文字列が必要です",
        ))
    }
}

#[derive(Debug, Clone, Copy)]
enum Number {
    Integer(i64),
    Float(f64),
}

impl Number {
    fn to_value(self) -> Value {
        match self {
            Number::Integer(i) => Value::Integer(i),
            Number::Float(f) => Value::Float(f),
        }
    }

    fn promote(self, other: Number) -> (Number, Number) {
        match (self, other) {
            (Number::Float(a), Number::Float(b)) => (Number::Float(a), Number::Float(b)),
            (Number::Float(a), Number::Integer(b)) => (Number::Float(a), Number::Float(b as f64)),
            (Number::Integer(a), Number::Float(b)) => (Number::Float(a as f64), Number::Float(b)),
            (Number::Integer(a), Number::Integer(b)) => (Number::Integer(a), Number::Integer(b)),
        }
    }

    fn is_zero(self) -> bool {
        match self {
            Number::Integer(i) => i == 0,
            Number::Float(f) => f == 0.0,
        }
    }
}

fn numeric_fold(
    _runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
    start: Option<Number>,
    op: impl Fn(Number, Number) -> Result<Number, EvalError>,
) -> Result<Value, EvalError> {
    ensure_min_arity(args, 1)?;
    let mut iter = args.iter();
    let first = if let Some(initial) = start {
        initial
    } else {
        expect_number(iter.next().unwrap())?
    };
    let mut acc = first;
    for value in iter {
        let rhs = expect_number(value)?;
        let (left, right) = acc.promote(rhs);
        acc = op(left, right)?;
    }
    Ok(acc.to_value())
}

fn numeric_add(
    runtime: &mut RuntimeState,
    env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    numeric_fold(
        runtime,
        env,
        args,
        Some(Number::Integer(0)),
        |acc, rhs| match (acc, rhs) {
            (Number::Integer(a), Number::Integer(b)) => Ok(Number::Integer(a + b)),
            (Number::Float(a), Number::Float(b)) => Ok(Number::Float(a + b)),
            (Number::Float(a), Number::Integer(b)) => Ok(Number::Float(a + b as f64)),
            (Number::Integer(a), Number::Float(b)) => Ok(Number::Float(a as f64 + b)),
        },
    )
}

fn numeric_sub(
    runtime: &mut RuntimeState,
    env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_min_arity(args, 1)?;
    if args.len() == 1 {
        return match expect_number(&args[0])? {
            Number::Integer(i) => Ok(Value::Integer(-i)),
            Number::Float(f) => Ok(Value::Float(-f)),
        };
    }
    let first = expect_number(&args[0])?;
    numeric_fold(runtime, env, &args[1..], Some(first), |acc, rhs| {
        match (acc, rhs) {
            (Number::Integer(a), Number::Integer(b)) => Ok(Number::Integer(a - b)),
            (Number::Float(a), Number::Float(b)) => Ok(Number::Float(a - b)),
            (Number::Float(a), Number::Integer(b)) => Ok(Number::Float(a - b as f64)),
            (Number::Integer(a), Number::Float(b)) => Ok(Number::Float(a as f64 - b)),
        }
    })
}

fn numeric_mul(
    runtime: &mut RuntimeState,
    env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    numeric_fold(
        runtime,
        env,
        args,
        Some(Number::Integer(1)),
        |acc, rhs| match (acc, rhs) {
            (Number::Integer(a), Number::Integer(b)) => Ok(Number::Integer(a * b)),
            (Number::Float(a), Number::Float(b)) => Ok(Number::Float(a * b)),
            (Number::Float(a), Number::Integer(b)) => Ok(Number::Float(a * b as f64)),
            (Number::Integer(a), Number::Float(b)) => Ok(Number::Float(a as f64 * b)),
        },
    )
}

fn numeric_div(
    _runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_min_arity(args, 1)?;
    let mut iter = args.iter();
    let mut acc = if let Some(first) = iter.next() {
        expect_number(first)?
    } else {
        return Ok(Value::Integer(1));
    };
    for arg in iter {
        let rhs = expect_number(arg)?;
        if rhs.is_zero() {
            return Err(EvalError::new(
                EvalErrorKind::DivisionByZero,
                None,
                "0 で除算できません",
            ));
        }
        let (left, right) = acc.promote(rhs);
        acc = match (left, right) {
            (Number::Integer(a), Number::Integer(b)) => Number::Integer(a / b),
            (Number::Float(a), Number::Float(b)) => Number::Float(a / b),
            (Number::Float(a), Number::Integer(b)) => Number::Float(a / b as f64),
            (Number::Integer(a), Number::Float(b)) => Number::Float(a as f64 / b),
        };
    }
    Ok(acc.to_value())
}

macro_rules! numeric_compare {
    ($name:ident, $op:tt) => {
        fn $name(_runtime: &mut RuntimeState, _env: EnvHandle, args: &[Value]) -> Result<Value, EvalError> {
            ensure_arity(args, 2)?;
            let lhs = expect_number(&args[0])?;
            let rhs = expect_number(&args[1])?;
            let (l, r) = lhs.promote(rhs);
            let result = match (l, r) {
                (Number::Integer(a), Number::Integer(b)) => a $op b,
                (Number::Float(a), Number::Float(b)) => a $op b,
                (Number::Float(a), Number::Integer(b)) => a $op b as f64,
                (Number::Integer(a), Number::Float(b)) => (a as f64) $op b,
            };
            Ok(Value::Boolean(result))
        }
    };
}

numeric_compare!(numeric_eq, ==);
numeric_compare!(numeric_lt, <);
numeric_compare!(numeric_lte, <=);
numeric_compare!(numeric_gt, >);
numeric_compare!(numeric_gte, >=);

fn numeric_abs(
    _runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    match expect_number(&args[0])? {
        Number::Integer(i) => Ok(Value::Integer(i.abs())),
        Number::Float(f) => Ok(Value::Float(f.abs())),
    }
}

fn numeric_floor(
    _runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    match expect_number(&args[0])? {
        Number::Integer(i) => Ok(Value::Integer(i)),
        Number::Float(f) => Ok(Value::Float(f.floor())),
    }
}

fn numeric_ceil(
    _runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    match expect_number(&args[0])? {
        Number::Integer(i) => Ok(Value::Integer(i)),
        Number::Float(f) => Ok(Value::Float(f.ceil())),
    }
}

fn boolean_not(
    _runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    if let Value::Boolean(b) = args[0] {
        Ok(Value::Boolean(!b))
    } else {
        Err(EvalError::new(
            EvalErrorKind::TypeMismatch {
                expected: "boolean",
                found: args[0].type_name(),
            },
            None,
            "真偽値が必要です",
        ))
    }
}

fn primitive_print(
    runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    let text = value_to_string(runtime, &args[0]);
    runtime.emit_message(text);
    Ok(Value::Unit)
}

fn primitive_type_of(
    runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    let ty = args[0].type_name();
    Ok(runtime.alloc_string_value(ty.to_string()))
}

fn primitive_string_append(
    runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_min_arity(args, 1)?;
    let mut result = String::new();
    for arg in args {
        result.push_str(expect_string(runtime, arg)?);
    }
    Ok(runtime.alloc_string_value(result))
}

fn primitive_string_length(
    runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 1)?;
    let s = expect_string(runtime, &args[0])?;
    Ok(Value::Integer(s.chars().count() as i64))
}

fn primitive_bind_key(
    runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 2)?;
    let key_sequence = expect_string(runtime, &args[0])?.to_string();
    let command_name = expect_string(runtime, &args[1])?.to_string();
    let host = runtime.host_mut().ok_or_else(|| {
        EvalError::new(
            EvalErrorKind::Runtime("ホストが未設定です".into()),
            None,
            "ホストが未設定です",
        )
    })?;

    host.bind_key(&key_sequence, &command_name)
        .map_err(|msg| EvalError::new(EvalErrorKind::Runtime(msg.clone()), None, msg))?;

    Ok(Value::Unit)
}

fn primitive_set_gui_color(
    runtime: &mut RuntimeState,
    _env: EnvHandle,
    args: &[Value],
) -> Result<Value, EvalError> {
    ensure_arity(args, 2)?;
    let component = expect_string(runtime, &args[0])?.to_string();
    let color = expect_string(runtime, &args[1])?.to_string();
    let host = runtime.host_mut().ok_or_else(|| {
        EvalError::new(
            EvalErrorKind::Runtime("ホストが未設定です".into()),
            None,
            "ホストが未設定です",
        )
    })?;

    host.set_gui_color(&component, &color)
        .map_err(|msg| EvalError::new(EvalErrorKind::Runtime(msg.clone()), None, msg))?;

    Ok(Value::Unit)
}
