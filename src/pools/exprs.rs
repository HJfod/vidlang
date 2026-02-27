use std::sync::{Arc, RwLock};

use slotmap::{SlotMap, new_key_type};

use crate::ast::expr::Expr;

new_key_type! { pub struct ExprId; }

#[derive(Debug, Clone)]
pub struct Exprs {
    map: Arc<RwLock<SlotMap<ExprId, Expr>>>,
}

impl Exprs {
    pub fn new() -> Self {
        Self { map: Arc::new(RwLock::new(SlotMap::with_key())) }
    }
    pub fn add(&self, expr: Expr) -> ExprId {
        self.map.write().unwrap().insert(expr)
    }
    /// Note: do NOT call `Exprs::add` here, as that'll deadlock!
    pub fn exec<T>(&self, id: ExprId, executor: impl Fn(&Expr) -> T) -> T {
        executor(self.map.read().unwrap().get(id).unwrap())
    }
}
