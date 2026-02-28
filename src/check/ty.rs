use std::collections::HashMap;

use crate::{ast::expr::Expr, pools::{codebase::{Codebase, ModId}, exprs::ExprId, names::{NameId, Names}}};

pub enum Ty {
    Bool,
    Int,
    Float,
    Duration,
    String,
    Tuple(Vec<Ty>),
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

pub enum ConstValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Duration(f64),
    String(String),
    Tuple(Vec<ConstValue>),
}

pub enum Item {
    Constant(NameId, ExprId, ConstValue),
    Module {
        name: NameId,
        definition: ModId,
        items: HashMap<NameId, Item>,
    },
}

pub struct Checker {
    root_module: Item,
}

impl Checker {
    pub fn new(codebase: &Codebase, names: Names) -> Self {
        Self { root_module: Self::make_submodule(codebase, names, codebase.root()) }
    }
    fn make_submodule(codebase: &Codebase, names: Names, id: ModId) -> Item {
        let mut items = HashMap::new();
        for sub in codebase.submodules(id) {
            items.insert(
                names.add(&codebase.fetch(sub).name()), 
                Self::make_submodule(codebase, names.clone(), sub)
            );
        }
        Item::Module {
            name: names.add(&codebase.fetch(id).name()),
            definition: id,
            items,
        }
    }
}
