
use crate::{
    ast::expr::{Expr, Ident},
    entities::names::MISSING_NAME,
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_block(tokens: &mut Tokens) -> Self {
        let tk = tokens.expect_bracketed(BracketType::Braces);
        if let Token::Bracketed(_, mut content, span) = tk {
            let exprs = Expr::parse_semicolon_list(Expr::parse, &mut content);
            Self::Block(exprs, span)
        }
        else {
            Self::Ident(Ident(MISSING_NAME, tk.span()))
        }
    }
    pub(super) fn try_parse_control_flow(tokens: &mut Tokens) -> Option<Self> {
        let start = tokens.start();

        if tokens.peek_and_expect_symbol(Symbol::If) {
            let clause = Expr::parse(tokens).into();
            let truthy = Expr::parse_block(tokens).into();
            let falsy = tokens.peek_and_expect_symbol(Symbol::Else)
                .then(|| Expr::parse_block(tokens).into());
            return Some(Self::If { clause, truthy, falsy, span: tokens.span_from(start) });
        }
        if tokens.peek_bracketed(BracketType::Brackets) {
            return Some(Self::parse_block(tokens));
        }

        None
    }
}
