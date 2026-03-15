use std::{collections::HashMap, fmt::Display};

use crate::{
    ast::expr::{Expr, StringComp},
    check::checker::Checker,
    codebase::Codebase,
    pools::{exprs::ExprId, items::ItemId, messages::Message, modules::ModId, names::NameId}
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TupleFieldTy {
    /// The bool is if the field has a default value
    Field(NameId, Ty, bool),
    Enum(Vec<(NameId, Ty)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Int,
    Float,
    Duration,
    String,
    Color([u8; 4]),
    Function(Box<Ty>, Box<Ty>),
    Alias {
        name: NameId,
        of: Box<Ty>,
        /// Newtypes are never implicitly convertible to their target type
        is_newtype: bool,
    },
    Optional(Box<Ty>),
    Ref(Box<Ty>),
    List(Box<Ty>),
    Tuple(Vec<TupleFieldTy>),
    /// Type whose value is not yet resolved
    Undecided,
    /// Type produced by non-exhaustive constructs (i.e. value is never assignable)
    NonExhaustive(Box<Ty>, ExprId),
}

impl Ty {
    pub fn name(&self, codebase: &Codebase) -> String {
        match self {
            Self::Bool => String::from("bool"),
            Self::Int => String::from("int"),
            Self::Float => String::from("float"),
            Self::Duration => String::from("duration"),
            Self::String => String::from("string"),
            Self::Color(..) => String::from("color"),
            Self::Function(p, r) => format!("{} -> {}", p.name(codebase), r.name(codebase)),
            Self::Alias { name, of, is_newtype } => match is_newtype {
                true => codebase.names.get(*name).to_string(),
                false => format!("{} ({})", codebase.names.get(*name), of.reduce().name(codebase)),
            }
            Self::Optional(ty) => format!("{}?", ty.name(codebase)),
            Self::Ref(ty) => format!("ref {}", ty.name(codebase)),
            Self::List(ty) => format!("[{}]", ty.name(codebase)),
            Self::Tuple(fields) => format!(
                "({})",
                fields.iter().map(|f| match f {
                    TupleFieldTy::Field(name, ty, def) => format!(
                        "{}: {}{}",
                        codebase.names.get(*name),
                        ty.name(codebase),
                        if *def { " = default" } else { "" }
                    ),
                    TupleFieldTy::Enum(variants) => format!(
                        "enum {{ {} }}",
                        variants.iter().map(|(name, ty)| format!(
                            "{}: {}",
                            codebase.names.get(*name),
                            ty.name(codebase),
                        )).collect::<Vec<_>>().join(", ")
                    ),
                }).collect::<Vec<_>>().join(", ")
            ),
            Self::Undecided => String::from("<unknown>"),
            Self::NonExhaustive(ty, _) => format!("<non-exhaustive {}>", ty.name(codebase)),
        }
    }

    pub fn reduce(&self) -> Ty {
        match self {
            Self::Alias { name: _, of, is_newtype } => match is_newtype {
                true => self.clone(),
                false => *of.clone(),
            },
            _ => self.clone(),
        }
    }

    pub fn convert_to(&self, into: &Ty, codebase: &mut Codebase) -> Ty {
        // `let a = if { .. }`
        if let Ty::NonExhaustive(ty, e) = self {
            codebase.messages.add(Message::new_error(
                format!("cannot convert {} to {}", ty.name(codebase), into.name(codebase)),
                codebase.exprs.get(*e).span()
            ));
            return into.clone();
        }
        // `if { .. } = a`
        if let Ty::NonExhaustive(ty, e) = into {
            codebase.messages.add(Message::new_error(
                "non-exhaustive constructs can not be assigned to",
                codebase.exprs.get(*e).span()
            ));
            return *ty.clone();
        }
        // If these are the exact same type then just return the type we're converting into
        if self == into {
            return into.clone();
        }
        match (self, into) {
        }
    }
}

#[derive(Debug)]
pub enum Item {
    Constant(NameId, ExprId, Ty),
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

impl Expr {
    pub fn check(&self, checker: &mut Checker, codebase: &mut Codebase) -> Ty {
        match self {
            Self::None(..) => Ty::Optional(Box::from(Ty::Undecided)),
            Self::Bool(..) => Ty::Bool,
            Self::Int(..) => Ty::Int,
            Self::Float(..) => Ty::Float,
            Self::Duration(..) => Ty::Duration,
            Self::String(comps, _) => {
                comps.iter().for_each(|c| match c {
                    StringComp::String(_) => {},
                    StringComp::Expr(e) => {
                        codebase.exprs.get(*e).check(checker, codebase)
                    }
                });
                Ty::String
            }
        }
    }
}
