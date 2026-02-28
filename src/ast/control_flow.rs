
use crate::{
    ast::expr::{Expr, Parser},
    pools::exprs::ExprId,
    tokens::token::{BracketType, Symbol, Token}
};

impl Expr {
    pub(super) fn parse_block(parser: &mut Parser) -> ExprId {
        let tk = parser.tokens.expect_bracketed(BracketType::Braces);
        if let Token::Bracketed(_, mut content, span) = tk {
            let list = Expr::parse_semicolon_expr_list(&mut parser.fork(&mut content), false);
            parser.exprs.lock_mut().add(Self::Block(list, span))
        }
        else {
            parser.exprs.lock_mut().add(Self::Ident(parser.tokens.names.lock_mut().missing_path(tk.span())))
        }
    }
    pub(super) fn try_parse_control_flow(parser: &mut Parser)
     -> Option<ExprId>
    {
        let start = parser.tokens.start();

        if parser.tokens.peek_and_expect_symbol(Symbol::If) {
            let clause = Expr::parse(parser);
            let truthy = Expr::parse_block(parser);
            let falsy = parser.tokens.peek_and_expect_symbol(Symbol::Else)
                .then(|| Expr::parse_block(parser));
            return Some(parser.exprs.lock_mut().add(Self::If {
                clause, truthy, falsy,
                span: parser.tokens.span_from(start)
            }));
        }
        if parser.tokens.peek_bracketed(BracketType::Brackets) {
            return Some(Self::parse_block(parser));
        }
        if parser.tokens.peek_and_expect_symbol(Symbol::Await) {
            let expr = Expr::parse(parser);
            return Some(parser.exprs.lock_mut().add(Self::Await(expr, parser.tokens.span_from(start))));
        }
        if parser.tokens.peek_and_expect_symbol(Symbol::Return) {
            // Don't parse return value if there is a separator or eof coming
            let no_expr = parser.tokens.peek_symbol(Symbol::Semicolon) ||
                parser.tokens.peek_symbol(Symbol::Comma) ||
                parser.tokens.peek().is_none();
            let expr = (!no_expr).then(|| Expr::parse(parser));
            return Some(parser.exprs.lock_mut().add(Self::Return(expr, parser.tokens.span_from(start))));
        }

        None
    }
}
