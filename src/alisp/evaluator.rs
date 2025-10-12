use crate::alisp::ast::Expr;
use crate::alisp::error::{EvalError, EvalErrorKind};
use crate::alisp::primitives::PrimitiveRegistry;
use crate::alisp::reader;
use crate::alisp::runtime::{
    closure_ref, collect, define_symbol, extend_env, lookup_env, make_closure, make_rooted_env,
    maybe_collect, set_symbol, value_to_string, Closure, EnvHandle, Function, RuntimeState, Value,
};
use crate::alisp::symbol::{SymbolId, SymbolInterner};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EvalOutcome {
    pub value: Value,
    pub display: String,
    pub messages: Vec<String>,
}

pub struct Interpreter {
    runtime: RuntimeState,
    global_env: EnvHandle,
    specials: SpecialForms,
    _primitives: PrimitiveRegistry,
    load_paths: Vec<PathBuf>,
}

#[derive(Clone, Copy)]
struct SpecialForms {
    define: SymbolId,
    lambda: SymbolId,
    let_form: SymbolId,
    if_form: SymbolId,
    begin: SymbolId,
    set_bang: SymbolId,
    and_form: SymbolId,
    or_form: SymbolId,
    load: SymbolId,
}

impl SpecialForms {
    fn new(interner: &mut SymbolInterner) -> Self {
        Self {
            define: interner.intern("define"),
            lambda: interner.intern("lambda"),
            let_form: interner.intern("let"),
            if_form: interner.intern("if"),
            begin: interner.intern("begin"),
            set_bang: interner.intern("set!"),
            and_form: interner.intern("and"),
            or_form: interner.intern("or"),
            load: interner.intern("load"),
        }
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let mut runtime = RuntimeState::new();
        let specials = SpecialForms::new(&mut runtime.interner);
        let global_env = make_rooted_env(&mut runtime);
        let primitives = PrimitiveRegistry::install(&mut runtime, global_env);
        let mut load_paths = Vec::new();
        if let Ok(dir) = std::env::current_dir() {
            load_paths.push(dir);
        }
        Self { runtime, global_env, specials, _primitives: primitives, load_paths }
    }

    pub fn set_load_root<P: Into<PathBuf>>(&mut self, root: P) {
        self.load_paths.clear();
        self.load_paths.push(root.into());
    }

    pub fn eval_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), EvalError> {
        let candidate = path.as_ref();
        let resolved = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.load_paths
                .last()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(candidate)
        };

        let source = fs::read_to_string(&resolved).map_err(|err| {
            EvalError::new(
                EvalErrorKind::Runtime(format!("ファイルを読み込めませんでした: {}", err)),
                None,
                format!("ファイルを読み込めませんでした: {}", err),
            )
        })?;

        let parent = resolved.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        self.load_paths.push(parent);
        let _ = self.eval_source(&source)?;
        self.load_paths.pop();
        Ok(())
    }

    pub fn eval(&mut self, source: &str) -> Result<EvalOutcome, EvalError> {
        self.eval_source(source)
    }

    pub fn runtime_mut(&mut self) -> &mut RuntimeState {
        &mut self.runtime
    }

    pub fn runtime(&self) -> &RuntimeState {
        &self.runtime
    }

    fn eval_source(&mut self, source: &str) -> Result<EvalOutcome, EvalError> {
        let forms = reader::parse(source, &mut self.runtime.interner).map_err(EvalError::from_reader)?;
        self.eval_forms(forms, self.global_env)
    }

    fn eval_forms(&mut self, forms: Vec<Expr>, env: EnvHandle) -> Result<EvalOutcome, EvalError> {
        let mut last_value = Value::Unit;
        for form in forms {
            last_value = self.eval_expr(&form, env)?;
        }
        let display = value_to_string(&self.runtime, &last_value);
        let messages = self.runtime.drain_messages();
        collect(&mut self.runtime, &[last_value.clone()], &[self.global_env]);
        Ok(EvalOutcome { value: last_value, display, messages })
    }

    fn eval_expr(&mut self, expr: &Expr, env: EnvHandle) -> Result<Value, EvalError> {
        match expr {
            Expr::Integer(i) => Ok(Value::Integer(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::String(s) => Ok(self.runtime.alloc_string_value(s.clone())),
            Expr::Symbol(sym) => lookup_env(&self.runtime, env, *sym)
                .ok_or_else(|| EvalError::new(EvalErrorKind::NameNotFound(*sym), None, format!("未定義のシンボル: {}", self.runtime.resolve(*sym).unwrap_or("<unknown>")))),
            Expr::List(list) => self.eval_list(list, env),
        }
    }

    fn eval_list(&mut self, list: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if list.is_empty() {
            return Ok(Value::Unit);
        }
        let head = &list[0];
        if let Some(sym) = head.as_symbol() {
            if sym == self.specials.define {
                return self.eval_define(&list[1..], env);
            }
            if sym == self.specials.lambda {
                return self.eval_lambda(&list[1..], env);
            }
            if sym == self.specials.let_form {
                return self.eval_let(&list[1..], env);
            }
            if sym == self.specials.if_form {
                return self.eval_if(&list[1..], env);
            }
            if sym == self.specials.begin {
                return self.eval_begin(&list[1..], env);
            }
            if sym == self.specials.set_bang {
                return self.eval_set(&list[1..], env);
            }
            if sym == self.specials.and_form {
                return self.eval_and(&list[1..], env);
            }
            if sym == self.specials.or_form {
                return self.eval_or(&list[1..], env);
            }
            if sym == self.specials.load {
                return self.eval_load(&list[1..], env);
            }
        }
        self.eval_call(list, env)
    }

    fn eval_define(&mut self, tail: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if tail.is_empty() {
            return Err(EvalError::new(EvalErrorKind::InvalidDefineTarget, None, "define の書式が不正です"));
        }
        match &tail[0] {
            Expr::Symbol(name) => {
                if tail.len() != 2 {
                    return Err(EvalError::new(EvalErrorKind::InvalidDefineTarget, None, "define は1つの式を受け取ります"));
                }
                let value = self.eval_expr(&tail[1], env)?;
                define_symbol(&mut self.runtime, env, *name, value);
                Ok(Value::Unit)
            }
            Expr::List(items) if !items.is_empty() => {
                let fn_name = match &items[0] {
                    Expr::Symbol(sym) => *sym,
                    _ => return Err(EvalError::new(EvalErrorKind::InvalidDefineTarget, None, "関数名が不正です")),
                };
                let mut params = Vec::new();
                for param in &items[1..] {
                    match param {
                        Expr::Symbol(sym) => params.push(*sym),
                        _ => {
                            return Err(EvalError::new(EvalErrorKind::InvalidDefineTarget, None, "引数名はシンボルである必要があります"));
                        }
                    }
                }
                let body: Vec<Expr> = tail[1..].to_vec();
                let closure = make_closure(&mut self.runtime, params, body, env);
                define_symbol(&mut self.runtime, env, fn_name, Value::Function(Function::Lambda(closure)));
                Ok(Value::Unit)
            }
            _ => Err(EvalError::new(EvalErrorKind::InvalidDefineTarget, None, "define の左辺が不正です")),
        }
    }

    fn eval_lambda(&mut self, tail: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if tail.len() < 2 {
            return Err(EvalError::new(EvalErrorKind::Runtime("lambda の書式が不正です".into()), None, "lambda の書式が不正です"));
        }
        let params_expr = &tail[0];
        let body = tail[1..].to_vec();
        let mut params = Vec::new();
        if let Expr::List(items) = params_expr {
            for item in items {
                match item {
                    Expr::Symbol(sym) => params.push(*sym),
                    _ => {
                        return Err(EvalError::new(EvalErrorKind::Runtime("引数名はシンボルである必要があります".into()), None, "引数名はシンボルである必要があります"));
                    }
                }
            }
        } else {
            return Err(EvalError::new(EvalErrorKind::Runtime("lambda の引数がリストではありません".into()), None, "lambda の引数がリストではありません"));
        }
        let closure = make_closure(&mut self.runtime, params, body, env);
        Ok(Value::Function(Function::Lambda(closure)))
    }

    fn eval_let(&mut self, tail: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if tail.is_empty() {
            return Err(EvalError::new(EvalErrorKind::InvalidLetBinding, None, "let の書式が不正です"));
        }
        let bindings_expr = &tail[0];
        let body_exprs = &tail[1..];
        let mut bindings = Vec::new();
        if let Expr::List(entries) = bindings_expr {
            for entry in entries {
                match entry {
                    Expr::List(pair) if pair.len() == 2 => {
                        let name = match &pair[0] {
                            Expr::Symbol(sym) => *sym,
                            _ => {
                                return Err(EvalError::new(EvalErrorKind::InvalidLetBinding, None, "let の変数名が不正です"));
                            }
                        };
                        let value = self.eval_expr(&pair[1], env)?;
                        bindings.push((name, value));
                    }
                    _ => {
                        return Err(EvalError::new(EvalErrorKind::InvalidLetBinding, None, "let の束縛形式が不正です"));
                    }
                }
            }
        } else {
            return Err(EvalError::new(EvalErrorKind::InvalidLetBinding, None, "let の束縛部はリストである必要があります"));
        }
        let new_env = extend_env(&mut self.runtime, env, bindings);
        self.eval_begin(body_exprs, new_env)
    }

    fn eval_if(&mut self, tail: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if tail.len() != 3 {
            return Err(EvalError::new(EvalErrorKind::Runtime("if の書式が不正です".into()), None, "if の書式が不正です"));
        }
        let cond = self.eval_expr(&tail[0], env)?;
        if cond.is_truthy() {
            self.eval_expr(&tail[1], env)
        } else {
            self.eval_expr(&tail[2], env)
        }
    }

    fn eval_begin(&mut self, exprs: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        let mut last = Value::Unit;
        for expr in exprs {
            last = self.eval_expr(expr, env)?;
        }
        Ok(last)
    }

    fn eval_set(&mut self, tail: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if tail.len() != 2 {
            return Err(EvalError::new(EvalErrorKind::Runtime("set! の書式が不正です".into()), None, "set! の書式が不正です"));
        }
        let symbol = match &tail[0] {
            Expr::Symbol(sym) => *sym,
            _ => return Err(EvalError::new(EvalErrorKind::Runtime("set! の対象はシンボルである必要があります".into()), None, "set! の対象はシンボルである必要があります")),
        };
        let value = self.eval_expr(&tail[1], env)?;
        set_symbol(&mut self.runtime, env, symbol, value)?;
        Ok(Value::Unit)
    }

    fn eval_and(&mut self, exprs: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        let mut last = Value::Boolean(true);
        for expr in exprs {
            last = self.eval_expr(expr, env)?;
            if !last.is_truthy() {
                return Ok(Value::Boolean(false));
            }
        }
        Ok(Value::Boolean(last.is_truthy()))
    }

    fn eval_or(&mut self, exprs: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        for expr in exprs {
            let value = self.eval_expr(expr, env)?;
            if value.is_truthy() {
                return Ok(Value::Boolean(true));
            }
        }
        Ok(Value::Boolean(false))
    }

    fn eval_load(&mut self, exprs: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        if exprs.len() != 1 {
            return Err(EvalError::new(
                EvalErrorKind::ArityMismatch { expected: 1, found: exprs.len() },
                None,
                "load は1つの引数が必要です",
            ));
        }

        let target = self.eval_expr(&exprs[0], env)?;
        let path = self.expect_string_value(&target)?;
        let candidate = PathBuf::from(path);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            self
                .load_paths
                .last()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(candidate)
        };

        let source = fs::read_to_string(&resolved).map_err(|err| {
            EvalError::new(
                EvalErrorKind::Runtime(format!("ファイルを読み込めませんでした: {}", err)),
                None,
                format!("ファイルを読み込めませんでした: {}", err),
            )
        })?;

        let parent = resolved.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        self.load_paths.push(parent);
        let forms = reader::parse(&source, &mut self.runtime.interner).map_err(EvalError::from_reader)?;
        let _ = self.eval_forms(forms, self.global_env)?;
        self.load_paths.pop();
        Ok(Value::Unit)
    }

    fn expect_string_value(&mut self, value: &Value) -> Result<String, EvalError> {
        match value {
            Value::String(handle) => Ok(self.runtime.heap.string_ref(*handle).to_string()),
            _ => Err(EvalError::new(
                EvalErrorKind::TypeMismatch { expected: "string", found: value.type_name() },
                None,
                "文字列が必要です",
            )),
        }
    }

    fn eval_call(&mut self, list: &[Expr], env: EnvHandle) -> Result<Value, EvalError> {
        let callee = self.eval_expr(&list[0], env)?;
        let mut args = Vec::new();
        for arg in &list[1..] {
            args.push(self.eval_expr(arg, env)?);
        }
        maybe_collect(&mut self.runtime, &args, &[env, self.global_env]);
        match callee {
            Value::Function(Function::Builtin(func)) => func(&mut self.runtime, env, &args),
            Value::Function(Function::Lambda(handle)) => {
                let closure = closure_ref(&self.runtime, handle).clone();
                self.apply_closure(closure, &args)
            }
            other => Err(EvalError::new(
                EvalErrorKind::TypeMismatch { expected: "function", found: other.type_name() },
                None,
                "関数ではない値を呼び出しました",
            )),
        }
    }

    fn apply_closure(&mut self, closure: Closure, args: &[Value]) -> Result<Value, EvalError> {
        if closure.params.len() != args.len() {
            return Err(EvalError::new(
                EvalErrorKind::ArityMismatch { expected: closure.params.len(), found: args.len() },
                None,
                "引数の個数が一致しません",
            ));
        }
        let bindings = closure.params.iter().cloned().zip(args.iter().cloned()).collect();
        let new_env = extend_env(&mut self.runtime, closure.env, bindings);
        let mut last = Value::Unit;
        for expr in &closure.body {
            last = self.eval_expr(expr, new_env)?;
        }
        Ok(last)
    }
}
