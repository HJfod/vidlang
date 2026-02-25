
use crate::{ast::expr::{Expr, Ident, TyExpr}, entities::{messages::Messages, names::MISSING_NAME}, tokens::{token::Symbol, tokenstream::Tokens}};

impl Expr {
    pub(super) fn parse_definition(tokens: &mut Tokens, messages: Messages) -> Self {
        let start = tokens.start();
        if tokens.peek_and_expect_symbol(Symbol::Let, messages.clone()) {
            let name = Expr::parse_name(tokens, messages.clone());
            let ty = tokens.peek_and_expect_symbol(Symbol::Colon, messages.clone())
                .then(|| TyExpr::parse(tokens, messages.clone()));
            let value = tokens.peek_and_expect_symbol(Symbol::Assign, messages.clone())
                .then(|| Box::from(Expr::parse(tokens, messages.clone())));
            return Expr::VarDef { name, ty, value, span: tokens.span_from(start) }
        }
        let span = tokens.expected("definition", messages);
        Self::Ident(Ident(MISSING_NAME, span))
    }
}

