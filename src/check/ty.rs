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
    package_roots: HashMap<String, Item>,
}

impl Checker {
    pub fn new(codebase: &Codebase, names: Names) -> Self {
        Self {
            package_roots: codebase.packages()
                .map(|p| (p.0.to_owned(), Self::make_submodule(codebase, names.clone(), p.0, p.1)))
                .collect()
        }
    }
    fn make_submodule(codebase: &Codebase, names: Names, name: &str, id: ModId) -> Item {
        let mut items = HashMap::new();
        for (name, sub) in codebase.get_submodules_for(id) {
            items.insert(
                names.add(name),
                Self::make_submodule(codebase, names.clone(), name, sub)
            );
        }
        Item::Module {
            name: names.add(&name),
            definition: id,
            items,
        }
    }
}
