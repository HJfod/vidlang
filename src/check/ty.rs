use std::{collections::HashMap, fmt::Display};
use crate::{
    ast::expr::{Expr, StringComp, Visibility}, check::checker::{CheckPhase, Checker}, codebase::{self, Codebase}, pools::{exprs::ExprId, items::ItemId, messages::Message, modules::{ModId, Span}, names::NameId}
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
    /// Type produced by invalid syntax / type errors
    Invalid,
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
            Self::Invalid => String::from("<invalid>"),
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
        codebase: &Codebase,
    ) -> Result<Ty, String> {
        let from = self.reduce(codebase);
        let into = into.reduce(codebase);

        // If either side is already invalid, then just return invalid (since 
        // we know a syntax or type error has already been produced earlier and 
        // it would be rude of us to spam the user with errors that all 
        // ultimately would be caused by the same mistake)
        if from == Ty::Invalid || into == Ty::Invalid {
            return Ok(Ty::Invalid);
        }

        // `let a = if { .. }`
        if matches!(from, Ty::NonExhaustive(..)) || matches!(into, Ty::NonExhaustive(..)) {
            return Err(String::from(
                "non-exhaustive constructs (such as those produced by if \
                statements without an else branch) can not be assigned"
            ));
        }

        // If either side is undecided, return the other
        if from == Ty::Undecided {
            return Ok(into);
        }
        if into == Ty::Undecided {
            return Ok(from);
        }

        // If these are the exact same type then the conversion is always fine
        if from == into {
            return Ok(into);
        }

        // Otherwise we have to do some more complex conversions
        match (from, into) {
            // `A -> B` is convertible to `C -> D` iff `C -> A` and `B -> D`
            // example: `int -> byte` can be passed to `byte -> int` because 
            // `byte -> int` is always valid
            (Ty::Function(fp, fr), Ty::Function(ip, ir)) => {
                let np = ip.convert_to(&fp, codebase)?;
                let nr = fr.convert_to(&ir, codebase)?;
                Ok(Ty::Function(Box::from(np), Box::from(nr)))
            }
            // Otherwise if we're here, the conversion is invalid
            (from, into) => {
                Err(format!("cannot convert from {} to {}", from.name(codebase), into.name(codebase)))
            }
        }
    }
}

#[derive(Debug)]
pub enum Item {
    Constant {
        visibility: Visibility,
        name: NameId,
        definition: ExprId,
        ty: Ty,
    },
    Module {
        visibility: Visibility,
        name: NameId,
        definition: ModId,
        items: HashMap<NameId, ItemId>,
        anon_items: Vec<ItemId>,
    },
    Function {
        visibility: Visibility,
        name: NameId,
        definition: ExprId,
        items: HashMap<NameId, ItemId>,
        anon_items: Vec<ItemId>,
        variables: Vec<(NameId, ExprId, Ty)>,
    },
    BlockScope {
        definition: ExprId,
        items: HashMap<NameId, ItemId>,
        anon_items: Vec<ItemId>,
        variables: Vec<(NameId, ExprId, Ty)>,
    },
}

impl Item {
    pub fn name(&self) -> Option<NameId> {
        match self {
            Self::Constant { name, .. } | Self::Function { name, .. } | Self::Module { name, .. } => Some(*name),
            Self::BlockScope { .. } => None
        }
    }
    pub fn span(&self, codebase: &Codebase) -> Span {
        match self {
            Self::Constant { definition, .. } | 
            Self::Function { definition, .. } | 
            Self::BlockScope { definition, .. } => codebase.exprs.get(*definition).span(),
            Self::Module { definition, .. } => codebase.full_span_for(*definition),
        }
    }

    pub fn get_subitems(&self) -> Vec<ItemId> {
        match self {
            Item::Constant { .. } => vec![],
            Item::Module { items, .. } => items.values().copied().collect(),
            Item::BlockScope { items, .. } => items.values().copied().collect(),
            Item::Function { items, .. } => items.values().copied().collect(),
        }
    }
}

impl<'cb> Checker<'cb> {
    fn ensure_convertible<F, S>(&mut self, from: &Ty, into: &Ty, msg: F) -> Ty
        where F: FnOnce() -> (S, Span), S: Display
    {
        // If we're in the discovery phase, skip this
        if self.discovering() {
            return Ty::Undecided;
        }
        match from.convert_to(into, self.codebase) {
            Ok(ty) => ty,
            Err(e) => {
                let (msg, span) = msg();
                self.codebase.messages.add(Message::new_error(msg, span).with_note(e, None));
                Ty::Invalid
            }
        }
    }
    fn check_and_ensure<F, S>(&mut self, from: ExprId, into: &Ty, msg: F) -> Ty
        where F: FnOnce() -> S, S: Display
    {
        let span = self.codebase.exprs.get(from).span();
        let from_ty = self.check_expr(from);
        self.ensure_convertible(&from_ty, into, || (msg(), span))
    }

    fn check_opt(&mut self, expr: Option<ExprId>) -> Ty {
        expr.map(|t| self.check_expr(t)).unwrap_or(Ty::Undecided)
    }

    pub(super) fn check_expr(&mut self, expr: ExprId) -> Ty {
        // I want to copy most fields (to avoid lifetime issues) so might aswell 
        // do this and add `ref` to the patterns that are actually not `Copy`
        match *self.codebase.exprs.get(expr) {
            Expr::None(..) => Ty::Optional(Box::from(Ty::Undecided)),
            Expr::Bool(..) => Ty::Bool,
            Expr::Int(..) => Ty::Int,
            Expr::Float(..) => Ty::Float,
            Expr::Duration(..) => Ty::Duration,
            Expr::String(ref comps, _) => {
                // Oh the silly things that lifetimes make me do
                let comp_ids = comps.iter().filter_map(|c| match c {
                    StringComp::String(_) => None,
                    StringComp::Expr(e) => Some(*e),
                }).collect::<Vec<_>>();
                for id in comp_ids {
                    self.check_and_ensure(id, &Ty::String, || "component in \
                        string interpolation is not convertible to string");
                }
                Ty::String
            }
            Expr::Ident(ref path) => match self.phase() {
                CheckPhase::Discovery => Ty::Undecided,
                CheckPhase::Check { .. } => todo!(),
            }
            Expr::DefaultValue(_) => Ty::Undecided,

            Expr::Var { visibility, name, ty, value, is_const, span: _ } => {
                let decl_ty = self.check_opt(ty);
                let value_ty = self.check_opt(value);
                let ty = self.ensure_convertible(
                    &decl_ty, &value_ty,
                    || ("value of constant does not match its type", name.1)
                );
                match self.phase() {
                    CheckPhase::Discovery => {
                        if is_const {
                            self.add_item(Item::Constant {
                                visibility,
                                name: name.0,
                                definition: expr,
                                ty,
                            });
                        }
                    }
                    CheckPhase::Check { name_hint: _ } => {
                        todo!()
                    }
                }
                Ty::Undecided
            }
            _ => todo!()
        }
    }
}
