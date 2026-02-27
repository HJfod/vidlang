use crate::{ast::expr::Expr, pools::names::NameId};

pub enum Ty {
    Bool,
    Int,
    Float,
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
    String(String),
    Tuple(Vec<ConstValue>),
}
