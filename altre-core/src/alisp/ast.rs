use crate::alisp::symbol::SymbolId;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Symbol(SymbolId),
    List(Vec<Expr>),
}

impl Expr {
    pub fn as_symbol(&self) -> Option<SymbolId> {
        match self {
            Expr::Symbol(id) => Some(*id),
            _ => None,
        }
    }
}
