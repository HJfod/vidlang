use crate::{ast::expr::{Ident, IdentPath}, pools::modules::Span, tokens::token::Symbol};
use string_interner::{self, StringInterner, backend::StringBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NameId(usize);

impl string_interner::Symbol for NameId {
    fn try_from_usize(index: usize) -> Option<Self> {
        Some(Self(index))
    }
    fn to_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct Names {
    // BufferBackend might be a good choice too since resolving the names will 
    // be rare and mostly for errors
    names: StringInterner<StringBackend<NameId>>,
}

impl Names {
    pub fn new() -> Self {
        Self { names: StringInterner::new() }
    }
    pub fn add(&mut self, name: &str) -> NameId {
        self.names.get_or_intern(name)
    }
    pub fn get(&self, id: NameId) -> &str {
        self.names.resolve(id).expect("Names has handed out an invalid NameId (somehow)")
    }

    pub fn missing(&mut self) -> NameId {
        self.add("<missing name>")
    }
    pub fn missing_path(&mut self, span: Span) -> IdentPath {
        IdentPath(vec![Ident(self.missing(), span)], span)
    }
    fn make_op_path(&mut self, func: &str, span: Span) -> IdentPath {
        IdentPath(vec![
            Ident(self.add("std"), span),
            Ident(self.add("ops"), span),
            Ident(self.add(func), span),
        ], span)
    }
    pub fn builtin_unop_name(&mut self, op: Symbol, span: Span) -> IdentPath {
        match op {
            // Unary plus is not real (since Rust also doesn't have it and I think 
            // they're base for doing so)
            Symbol::Minus => self.make_op_path("neg", span),
            Symbol::Exclamation => self.make_op_path("not", span),
            _ => panic!("invalid op passed to builtin_unop_name"),
        }
    }
    pub fn builtin_binop_name(&mut self, op: Symbol, span: Span) -> IdentPath {
        match op {
            Symbol::Power => self.make_op_path("power", span),

            Symbol::Plus => self.make_op_path("add", span),
            Symbol::Minus => self.make_op_path("sub", span),
            Symbol::Mul => self.make_op_path("mul", span),
            Symbol::Div => self.make_op_path("div", span),
            Symbol::Mod => self.make_op_path("modulo", span),

            Symbol::More => self.make_op_path("more", span),
            Symbol::Meq => self.make_op_path("meq", span),
            Symbol::Eq => self.make_op_path("eq", span),
            Symbol::Neq => self.make_op_path("neq", span),
            Symbol::Leq => self.make_op_path("leq", span),
            Symbol::Less => self.make_op_path("less", span),

            _ => panic!("invalid op passed to builtin_binop_name"),
        }
    }
}
