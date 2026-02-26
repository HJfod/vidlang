
use crate::{
    ast::expr::{Expr, ParseArgs, TyExpr},
    entities::messages::Message,
    tokens::{token::Symbol, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn try_parse_definition(tokens: &mut Tokens, args: ParseArgs) -> Option<Self> {
        let start = tokens.start();

        // Variable definition
        if tokens.peek_and_expect_symbol(Symbol::Let) {
            let name = tokens.expect_ident();
            let ty = tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| TyExpr::parse(tokens));
            let value = tokens.peek_and_expect_symbol(Symbol::Assign)
                .then(|| Box::from(Expr::parse(tokens, args)));
            return Some(Expr::VarDef { name, ty, value, span: tokens.span_from(start) })
        }
        
        None
    }
    pub(super) fn parse_definition(tokens: &mut Tokens, args: ParseArgs) -> Self {
        match Self::try_parse_definition(tokens, args) {
            Some(v) => v,
            None => {
                let start = tokens.start();
                // They probably tried to write an expr, so parsing one should 
                // result in less errors overall
                let bad_expr = Expr::parse(tokens, args);
                let span = tokens.span_from(start);
                tokens.messages().add(Message::new_error("only definitions may appear here", span));
                bad_expr
            }
        }
    }
}

