use std::sync::{Arc, Mutex};
use crate::tokens::token::Symbol;
use string_interner::{self, StringInterner, backend::StringBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameId(usize);

impl string_interner::Symbol for NameId {
    fn try_from_usize(index: usize) -> Option<Self> {
        Some(Self(index))
    }
    fn to_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Names {
    // BufferBackend might be a good choice too since resolving the names will 
    // be rare and mostly for errors
    names: Arc<Mutex<StringInterner<StringBackend<NameId>>>>,
}

impl Names {
    pub fn new() -> Self {
        Self {
            names: Arc::from(Mutex::new(StringInterner::new()))
        }
    }
    pub fn add(&self, name: &str) -> NameId {
        self.names.lock().unwrap()
            .get_or_intern(name)
    }
    pub fn fetch(&self, id: NameId) -> String {
        self.names.lock().unwrap()
            .resolve(id)
            .expect("Names has handed out an invalid NameId (somehow)")
            .to_string()
    }

    pub fn missing(&self) -> NameId {
        self.add("<missing name>")
    }
    pub fn builtin_unop_name(&self, op: Symbol) -> NameId {
        match op {
            // Unary plus is not real (since Rust also doesn't have it and I think 
            // they're base for doing so)
            Symbol::Minus => self.add("op_neg"),
            Symbol::Exclamation => self.add("op_sub"),
            _ => panic!("invalid op passed to builtin_unop_name"),
        }
    }
    pub fn builtin_binop_name(&self, op: Symbol) -> NameId {
        match op {
            Symbol::Power => self.add("op_power"),

            Symbol::Plus => self.add("op_add"),
            Symbol::Minus => self.add("op_sub"),
            Symbol::Mul => self.add("op_mul"),
            Symbol::Div => self.add("op_div"),
            Symbol::Mod => self.add("op_mod"),

            Symbol::More => self.add("op_more"),
            Symbol::Meq => self.add("op_meq"),
            Symbol::Eq => self.add("op_eq"),
            Symbol::Neq => self.add("op_neq"),
            Symbol::Leq => self.add("op_leq"),
            Symbol::Less => self.add("op_less"),

            _ => panic!("invalid op passed to builtin_binop_name"),
        }
    }
}
