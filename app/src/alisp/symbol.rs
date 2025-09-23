use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub(crate) usize);

impl SymbolId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct SymbolInterner {
    lookup: HashMap<String, SymbolId>,
    symbols: Vec<String>,
}

impl SymbolInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern<S: AsRef<str>>(&mut self, symbol: S) -> SymbolId {
        let sym = symbol.as_ref();
        if let Some(id) = self.lookup.get(sym) {
            *id
        } else {
            let id = SymbolId(self.symbols.len());
            self.symbols.push(sym.to_string());
            self.lookup.insert(sym.to_string(), id);
            id
        }
    }

    pub fn resolve(&self, id: SymbolId) -> Option<&str> {
        self.symbols.get(id.index()).map(|s| s.as_str())
    }
}
