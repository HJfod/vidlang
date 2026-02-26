use std::sync::{Arc, Mutex};

use crate::tokens::token::Symbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameId(usize);

pub static MISSING_NAME: NameId = NameId(0);

#[derive(Debug, Clone)]
pub struct Names {
    names: Arc<Mutex<Vec<String>>>,
}

impl Names {
    pub fn new() -> Self {
        Self {
            // First name is always MISSING_NAME
            names: Arc::from(Mutex::new(vec!["<missing name>".into()]))
        }
    }
    pub fn add(&self, name: &str) -> NameId {
        let mut names = self.names.lock().unwrap();
        if let Some((id, _)) = names.iter().enumerate().find(|n| n.1 == name) {
            return NameId(id);
        }
        names.push(name.to_string());
        NameId(names.len() - 1)
    }
    pub fn fetch(&self, id: NameId) -> String {
        self.names.lock().unwrap()
            .get(id.0)
            .expect("NamePool has apparently handed out an invalid NameId")
            .clone()
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
