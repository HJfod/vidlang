use slotmap::{SlotMap, new_key_type};
use crate::ast::expr::Expr;

new_key_type! { pub struct ExprId; }

#[derive(Debug)]
pub struct Exprs {
    map: SlotMap<ExprId, Expr>,
}

impl Exprs {
    pub fn new() -> Self {
        Self { map: SlotMap::with_key() }
    }
    pub fn add(&mut self, expr: Expr) -> ExprId {
        self.map.insert(expr)
    }
    pub fn get(&self, id: ExprId) -> &Expr {
        self.map.get(id).expect("Exprs has handed out an invalid ExprId")
    }
}
