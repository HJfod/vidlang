use std::collections::HashMap;

use crate::{
    entities::{messages::{self, Messages}, names::{NameId, Names}, src::{Codebase, ModId, Span}}, 
    tokens::tokenstream::Tokens
};

#[derive(Debug)]
pub enum StringComp {
    String(String),
    Expr(Expr),
}

#[derive(Debug)]
pub enum TyExpr {
    Named(NameId, Span),
}

#[derive(Debug)]
pub struct Ident(pub NameId, pub Span);

#[derive(Debug)]
pub enum Expr {
    // Literals
    Int(u64, Span),
    Float(f64, Span),
    String(Vec<StringComp>, Span),
    Ident(Ident),

    // Definitions
    VarDef {
        name: Ident,
        ty: Option<TyExpr>,
        value: Option<Box<Expr>>,
        span: Span,
    },

    // Control flow
    Block(Vec<Expr>, Span),
}

impl Expr {
    pub fn parse(tokens: &mut Tokens, messages: Messages) -> Self {
        Self::parse_binop(tokens, messages)
    }
}

pub struct ASTs {
    asts: HashMap<ModId, Vec<Expr>>,
}

impl ASTs {
    pub fn parse_all(codebase: &Codebase, names: Names, messages: Messages) -> ASTs {
        let mut ret = ASTs {
            asts: Default::default(),
        };
        for id in codebase.all_ids() {
            let tokens = &mut codebase.tokenize(id, names.clone(), messages.clone());
            let mut exprs = Vec::new();
            while tokens.peek().is_some() {
                exprs.push(Expr::parse_definition(tokens, messages.clone()));
            }
            ret.asts.insert(id, exprs);
        }
        ret
    }
}
