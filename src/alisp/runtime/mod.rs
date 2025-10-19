use crate::alisp::ast::Expr;
use crate::alisp::error::{EvalError, EvalErrorKind};
use crate::alisp::symbol::{SymbolId, SymbolInterner};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringHandle(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvHandle(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosureHandle(usize);

#[derive(Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(StringHandle),
    Function(Function),
    Unit,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::Boolean(_) => "boolean",
            Value::String(_) => "string",
            Value::Function(_) => "function",
            Value::Unit => "unit",
        }
    }

    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Boolean(false))
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::String(_) => write!(f, "<string>"),
            Value::Function(func) => write!(f, "{:?}", func),
            Value::Unit => write!(f, "()"),
        }
    }
}

#[derive(Clone)]
pub enum Function {
    Builtin(BuiltinFunction),
    Lambda(ClosureHandle),
}

pub type BuiltinFunction = fn(&mut RuntimeState, EnvHandle, &[Value]) -> Result<Value, EvalError>;

#[derive(Debug, Clone)]
pub struct Closure {
    pub params: Vec<SymbolId>,
    pub body: Vec<Expr>,
    pub env: EnvHandle,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub parent: Option<EnvHandle>,
    pub bindings: Vec<(SymbolId, Value)>,
}

#[derive(Debug)]
pub enum HeapObject {
    String(String),
    Env(Environment),
    Closure(Closure),
}

#[derive(Debug)]
struct HeapEntry {
    object: HeapObject,
    marked: bool,
}

#[derive(Debug)]
pub struct GcHeap {
    entries: Vec<Option<HeapEntry>>,
    allocated: usize,
    next_gc_threshold: usize,
}

impl GcHeap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            allocated: 0,
            next_gc_threshold: 128,
        }
    }

    pub fn alloc_string(&mut self, value: String) -> StringHandle {
        let handle = StringHandle(self.entries.len());
        self.entries.push(Some(HeapEntry {
            object: HeapObject::String(value),
            marked: false,
        }));
        self.allocated += 1;
        handle
    }

    pub fn alloc_env(&mut self, env: Environment) -> EnvHandle {
        let handle = EnvHandle(self.entries.len());
        self.entries.push(Some(HeapEntry {
            object: HeapObject::Env(env),
            marked: false,
        }));
        self.allocated += 1;
        handle
    }

    pub fn alloc_closure(&mut self, closure: Closure) -> ClosureHandle {
        let handle = ClosureHandle(self.entries.len());
        self.entries.push(Some(HeapEntry {
            object: HeapObject::Closure(closure),
            marked: false,
        }));
        self.allocated += 1;
        handle
    }

    pub fn string_ref(&self, handle: StringHandle) -> &str {
        match self.entries.get(handle.0).and_then(|e| e.as_ref()) {
            Some(HeapEntry {
                object: HeapObject::String(s),
                ..
            }) => s.as_str(),
            _ => panic!("invalid string handle"),
        }
    }

    pub fn env_ref(&self, handle: EnvHandle) -> &Environment {
        match self.entries.get(handle.0).and_then(|e| e.as_ref()) {
            Some(HeapEntry {
                object: HeapObject::Env(env),
                ..
            }) => env,
            _ => panic!("invalid env handle"),
        }
    }

    pub fn env_mut(&mut self, handle: EnvHandle) -> &mut Environment {
        match self.entries.get_mut(handle.0).and_then(|e| e.as_mut()) {
            Some(HeapEntry {
                object: HeapObject::Env(env),
                ..
            }) => env,
            _ => panic!("invalid env handle"),
        }
    }

    pub fn closure_ref(&self, handle: ClosureHandle) -> &Closure {
        match self.entries.get(handle.0).and_then(|e| e.as_ref()) {
            Some(HeapEntry {
                object: HeapObject::Closure(closure),
                ..
            }) => closure,
            _ => panic!("invalid closure handle"),
        }
    }

    pub fn closure_mut(&mut self, handle: ClosureHandle) -> &mut Closure {
        match self.entries.get_mut(handle.0).and_then(|e| e.as_mut()) {
            Some(HeapEntry {
                object: HeapObject::Closure(closure),
                ..
            }) => closure,
            _ => panic!("invalid closure handle"),
        }
    }

    pub fn maybe_collect(&mut self, roots: &[Value], env_roots: &[EnvHandle]) {
        if self.allocated < self.next_gc_threshold {
            return;
        }
        self.collect_garbage(roots, env_roots);
        self.next_gc_threshold = (self.entries.len().max(1)) * 2;
    }

    pub fn collect_garbage(&mut self, roots: &[Value], env_roots: &[EnvHandle]) {
        for entry in &mut self.entries {
            if let Some(e) = entry.as_mut() {
                e.marked = false;
            }
        }
        for env in env_roots {
            self.mark_env(*env);
        }
        for value in roots {
            self.mark_value(value);
        }
        for entry in &mut self.entries {
            if let Some(e) = entry {
                if !e.marked {
                    *entry = None;
                }
            }
        }
        self.allocated = self.entries.iter().filter(|e| e.is_some()).count();
    }

    fn mark_string(&mut self, handle: StringHandle) {
        if let Some(entry) = self.entries.get_mut(handle.0).and_then(|e| e.as_mut()) {
            if entry.marked {
                return;
            }
            entry.marked = true;
        }
    }

    fn mark_env(&mut self, handle: EnvHandle) {
        let parent: Option<EnvHandle>;
        let bindings: Vec<(SymbolId, Value)>;
        {
            let entry = match self.entries.get_mut(handle.0).and_then(|e| e.as_mut()) {
                Some(entry) => entry,
                None => return,
            };
            if entry.marked {
                return;
            }
            entry.marked = true;
            match &entry.object {
                HeapObject::Env(env) => {
                    parent = env.parent;
                    bindings = env.bindings.clone();
                }
                _ => return,
            }
        }
        if let Some(parent) = parent {
            self.mark_env(parent);
        }
        for (_, value) in bindings {
            self.mark_value(&value);
        }
    }

    fn mark_closure(&mut self, handle: ClosureHandle) {
        let env_handle: EnvHandle;
        {
            let entry = match self.entries.get_mut(handle.0).and_then(|e| e.as_mut()) {
                Some(entry) => entry,
                None => return,
            };
            if entry.marked {
                return;
            }
            entry.marked = true;
            env_handle = match &entry.object {
                HeapObject::Closure(closure) => closure.env,
                _ => return,
            };
        }
        self.mark_env(env_handle);
    }

    fn mark_value(&mut self, value: &Value) {
        match value {
            Value::String(handle) => self.mark_string(*handle),
            Value::Function(Function::Lambda(handle)) => self.mark_closure(*handle),
            Value::Function(Function::Builtin(_)) => {}
            Value::Integer(_) | Value::Float(_) | Value::Boolean(_) | Value::Unit => {}
        }
    }
}

pub trait HostBridge {
    fn bind_key(
        &mut self,
        key_sequence: &str,
        command_name: &str,
    ) -> std::result::Result<(), String>;
}

pub struct RuntimeState {
    pub heap: GcHeap,
    pub interner: SymbolInterner,
    pub messages: Vec<String>,
    host: Option<Box<dyn HostBridge>>,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            heap: GcHeap::new(),
            interner: SymbolInterner::new(),
            messages: Vec::new(),
            host: None,
        }
    }

    pub fn intern<S: AsRef<str>>(&mut self, sym: S) -> SymbolId {
        self.interner.intern(sym)
    }

    pub fn resolve(&self, id: SymbolId) -> Option<&str> {
        self.interner.resolve(id)
    }

    pub fn alloc_string_value(&mut self, value: String) -> Value {
        let handle = self.heap.alloc_string(value);
        Value::String(handle)
    }

    pub fn emit_message(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    pub fn drain_messages(&mut self) -> Vec<String> {
        std::mem::take(&mut self.messages)
    }

    pub fn set_host(&mut self, host: Box<dyn HostBridge>) {
        self.host = Some(host);
    }

    pub fn host_mut(&mut self) -> Option<&mut (dyn HostBridge + '_)> {
        if let Some(host) = self.host.as_mut() {
            Some(host.as_mut())
        } else {
            None
        }
    }
}

pub fn value_to_string(runtime: &RuntimeState, value: &Value) -> String {
    match value {
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format!("{}", f),
        Value::Boolean(true) => "#t".to_string(),
        Value::Boolean(false) => "#f".to_string(),
        Value::String(handle) => runtime.heap.string_ref(*handle).to_string(),
        Value::Function(Function::Builtin(_)) => "<builtin>".to_string(),
        Value::Function(Function::Lambda(_)) => "<lambda>".to_string(),
        Value::Unit => "()".to_string(),
    }
}

pub fn make_rooted_env(runtime: &mut RuntimeState) -> EnvHandle {
    runtime.heap.alloc_env(Environment {
        parent: None,
        bindings: Vec::new(),
    })
}

pub fn extend_env(
    runtime: &mut RuntimeState,
    parent: EnvHandle,
    bindings: Vec<(SymbolId, Value)>,
) -> EnvHandle {
    runtime.heap.alloc_env(Environment {
        parent: Some(parent),
        bindings,
    })
}

pub fn lookup_env(runtime: &RuntimeState, env: EnvHandle, symbol: SymbolId) -> Option<Value> {
    let mut current = Some(env);
    while let Some(handle) = current {
        let frame = runtime.heap.env_ref(handle);
        if let Some((_, value)) = frame.bindings.iter().find(|(name, _)| *name == symbol) {
            return Some(value.clone());
        }
        current = frame.parent;
    }
    None
}

pub fn define_symbol(runtime: &mut RuntimeState, env: EnvHandle, symbol: SymbolId, value: Value) {
    let frame = runtime.heap.env_mut(env);
    if let Some((_, slot)) = frame.bindings.iter_mut().find(|(name, _)| *name == symbol) {
        *slot = value;
    } else {
        frame.bindings.push((symbol, value));
    }
}

pub fn set_symbol(
    runtime: &mut RuntimeState,
    env: EnvHandle,
    symbol: SymbolId,
    value: Value,
) -> Result<(), EvalError> {
    let mut current = Some(env);
    while let Some(handle) = current {
        let frame = runtime.heap.env_mut(handle);
        if let Some((_, slot)) = frame.bindings.iter_mut().find(|(name, _)| *name == symbol) {
            *slot = value;
            return Ok(());
        }
        current = frame.parent;
    }
    Err(EvalError::new(
        EvalErrorKind::NameNotFound(symbol),
        None,
        format!(
            "未定義のシンボル: {}",
            runtime.resolve(symbol).unwrap_or("<unknown>")
        ),
    ))
}

pub fn make_closure(
    runtime: &mut RuntimeState,
    params: Vec<SymbolId>,
    body: Vec<Expr>,
    env: EnvHandle,
) -> ClosureHandle {
    runtime.heap.alloc_closure(Closure { params, body, env })
}

pub fn closure_ref(runtime: &RuntimeState, handle: ClosureHandle) -> &Closure {
    runtime.heap.closure_ref(handle)
}

pub fn collect(runtime: &mut RuntimeState, roots: &[Value], env_roots: &[EnvHandle]) {
    runtime.heap.collect_garbage(roots, env_roots);
}

pub fn maybe_collect(runtime: &mut RuntimeState, roots: &[Value], env_roots: &[EnvHandle]) {
    runtime.heap.maybe_collect(roots, env_roots);
}
impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Builtin(_) => write!(f, "<builtin>"),
            Function::Lambda(_) => write!(f, "<lambda>"),
        }
    }
}
