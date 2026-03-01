use std::collections::HashMap;

use crate::{ast::expr::Expr, check::ir::ConstValue, pools::{exprs::ExprId, items::ItemId, modules::ModId, names::NameId}};

#[derive(Debug)]
pub enum Ty {
    Bool,
    Int,
    Float,
    Duration,
    String,
    Function {
        params: Vec<(Ty, bool)>,
        return_ty: Box<Ty>,
    },
    Alias {
        name: NameId,
        of: Box<Ty>,
        /// Newtypes are never implicitly convertible to their target type
        is_newtype: bool
    },
    /// Type whose value is not yet resolved
    Undecided,
    /// Type produced by non-exhaustive constructs (i.e. value is never assignable)
    // todo: maybe put all exprs in a pool for stuff like this?
    NonExhaustive(Expr),
}

#[derive(Debug)]
pub enum Item {
    Constant(NameId, ExprId, ConstValue),
    Module {
        name: NameId,
        definition: ModId,
        items: HashMap<NameId, ItemId>,
    },
}

impl Item {
    pub fn get_subitems(&self) -> Vec<ItemId> {
        match self {
            Item::Constant(..) => vec![],
            Item::Module { items, .. } => items.values().copied().collect(),
        }
    }
}
