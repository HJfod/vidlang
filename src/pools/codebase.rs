use std::{collections::HashMap};

use crate::{
    ast::expr::{Ast, ParseArgs},
    pools::{exprs::Exprs, items::Items, messages::Messages, modules::{ModId, Modules, Span, SrcIterator}, names::Names},
    tokens::{token::Token, tokenstream::Tokens}
};

pub struct Codebase {
    pub modules: Modules,
    pub names: Names,
    pub messages: Messages,
    pub exprs: Exprs,
    pub items: Items,
    pub parsed_asts: HashMap<ModId, Ast>,
}

impl Codebase {
    pub fn new() -> Self {
        Self { 
            modules: Modules::new(),
            names: Names::new(),
            messages: Messages::new(),
            exprs: Exprs::new(),
            items: Items::new(),
            parsed_asts: HashMap::new()
        }
    }
    #[cfg(test)]
    pub fn new_with_test_package(name: &str, data: &str) -> (Self, ModId) {
        let mut ret = Self::new();
        let id = ret.modules.add_test_package(name, data);
        (ret, id)
    }

    pub fn tokenize_mod(&mut self, mod_id: ModId) -> Option<Tokens> {
        // Can't use `get()` here because overlapping borrows
        let mut iter = SrcIterator::new(mod_id, self.modules.get(mod_id).data.as_ref()?.chars());
        let mut tokens = Vec::new();
        while let Some(tk) = Token::parse(&mut iter, &mut self.names, &mut self.messages) {
            tokens.push(tk);
        }
        Some(Tokens::new(tokens, "eof", Span::zero(mod_id)))
    }
    pub fn parse_one(&mut self, id: ModId, args: ParseArgs) -> Option<&Ast> {
        // Having to re-get this is silly but I ran into borrow checker issues
        if self.parsed_asts.contains_key(&id) {
            return self.parsed_asts.get(&id);
        }
        let mut tokens = self.tokenize_mod(id)?;
        let parsed = Ast::parse(&mut tokens, self, args);
        self.parsed_asts.insert(id, parsed);
        self.parsed_asts.get(&id)
    }
    pub fn parse_all(&mut self, args: ParseArgs) {
        for id in self.modules.all() {
            self.parse_one(id, args);
        }
    }
}
