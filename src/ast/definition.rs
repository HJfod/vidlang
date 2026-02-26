
use crate::{
    ast::expr::{Expr, Ident, TyExpr},
    entities::{names::MISSING_NAME},
    tokens::{token::Symbol, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn try_parse_definition(tokens: &mut Tokens) -> Option<Self> {
        let start = tokens.start();

        // Variable definition
        if tokens.peek_and_expect_symbol(Symbol::Let) {
            let name = Expr::parse_name(tokens);
            let ty = tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| TyExpr::parse(tokens));
            let value = tokens.peek_and_expect_symbol(Symbol::Assign)
                .then(|| Box::from(Expr::parse(tokens)));
            return Some(Expr::VarDef { name, ty, value, span: tokens.span_from(start) })
        }
        
        None
    }
    pub(super) fn parse_definition(tokens: &mut Tokens) -> Self {
        match Self::try_parse_definition(tokens) {
            Some(v) => v,
            None => {
                let span = tokens.expected("definition");
                Expr::Ident(Ident(MISSING_NAME, span))
            }
        }
    }
}

