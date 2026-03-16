use std::collections::HashMap;
use crate::{
    ast::expr::{Expr, StringComp},
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
    /// Type produced by the `default` keyword, which assigns the default value 
    /// of the provided type
    AssignDefault,
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
                false => format!(
                    "{} ({})",
                    codebase.names.get(*name),
                    of.reduce(codebase).name(codebase)
                ),
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
            Self::AssignDefault => String::from("<default>"),
        }
    }

    pub fn reduce(&self, codebase: &Codebase) -> Ty {
        match self {
            Self::Alias { name: _, of, is_newtype } if !*is_newtype => *of.clone(),
            // `(T) == T`
            Self::Tuple(fields) if let [TupleFieldTy::Field(name, ty, false)] = &fields[..] => {
                if codebase.names.get(*name) == "0" {
                    return ty.clone();
                }
                self.clone()
            },
            _ => self.clone(),
        }
    }

    pub fn convert_to(
        &self,
        into: &Ty,
        phase: CheckPhase,
        codebase: &mut Codebase,
        disregard_tuple_names: bool,
    ) -> Ty {
        if phase == CheckPhase::Discovery {
            return into.clone();
        }
        let from = self.reduce(codebase);
        let into = into.reduce(codebase);
        // `let a = if { .. }`
        if let Ty::NonExhaustive(ty, e) = from {
            codebase.messages.add(Message::new_error(
                format!("cannot convert {} to {}", ty.name(codebase), into.name(codebase)),
                codebase.exprs.get(e).span()
            ));
            return into.clone();
        }
        // `if { .. } = a`
        if let Ty::NonExhaustive(ty, e) = into {
            codebase.messages.add(Message::new_error(
                "non-exhaustive constructs can not be assigned to",
                codebase.exprs.get(e).span()
            ));
            return *ty.clone();
        }
        // If these are the exact same type then the conversion is always fine
        if from == into {
            return into.clone();
        }
        match (self, into) {
            (Ty::Function(fp, fr), Ty::Function(ip, ir)) => {

            }
        }
    }
}

#[derive(Debug)]
pub enum ScopeKind {
    Function,
}

#[derive(Debug)]
pub enum Item {
    Constant(NameId, ExprId, Ty),
    Module {
        name: NameId,
        definition: ModId,
        items: HashMap<NameId, ItemId>,
    },
    Scope {
        kind: ScopeKind,
        name: Option<NameId>,
        definition: ExprId,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NameHint {
    Function,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CheckPhase {
    /// Find item names
    Discovery,
    /// Check types
    Check {
        name_hint: Option<NameHint>,
    },
}

pub fn check_expr(expr: ExprId, phase: CheckPhase, codebase: &mut Codebase) -> Ty {
    match codebase.exprs.get(expr) {
        Expr::None(..) => Ty::Optional(Box::from(Ty::Undecided)),
        Expr::Bool(..) => Ty::Bool,
        Expr::Int(..) => Ty::Int,
        Expr::Float(..) => Ty::Float,
        Expr::Duration(..) => Ty::Duration,
        Expr::String(comps, _) => {
            comps.iter().for_each(|c| match c {
                StringComp::String(_) => {},
                StringComp::Expr(e) => {
                    codebase.exprs.get(*e)
                        .check(phase, codebase)
                        .convert_to(&Ty::String, phase, codebase, false);
                }
            });
            Ty::String
        }
        Expr::Ident(path) => match phase {
            CheckPhase::Discovery => Ty::Undecided,
            CheckPhase::Check => todo!(),
        }
        Expr::DefaultValue(_) => Ty::Undecided,

        Expr::Var { visibility, name, ty, value, is_const, span } => match phase {
            CheckPhase::Discovery => {
                if !*is_const {
                    return Ty::Undecided;
                }
                let decl_ty = ty.map(|t| codebase.exprs.get(t).check(phase, codebase));
                todo!()
            }
            CheckPhase::Check => {
                todo!()
            }
        }
    }
}
