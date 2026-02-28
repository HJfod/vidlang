
use crate::{
    ast::expr::{Expr, ParseArgs},
    pools::exprs::{ExprId, Exprs},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_block(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs) -> ExprId {
        let tk = tokens.expect_bracketed(BracketType::Braces);
        if let Token::Bracketed(_, mut content, span) = tk {
            let list = Expr::parse_semicolon_expr_list(&mut content, false, exprs.clone(), args);
            exprs.add(Self::Block(list, span))
        }
        else {
            exprs.add(Self::Ident(tokens.names().missing_path(tk.span())))
        }
    }
    pub(super) fn try_parse_control_flow(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs)
     -> Option<ExprId>
    {
        let start = tokens.start();

        if tokens.peek_and_expect_symbol(Symbol::If) {
            let clause = Expr::parse(tokens, exprs.clone(), args);
            let truthy = Expr::parse_block(tokens, exprs.clone(), args);
            let falsy = tokens.peek_and_expect_symbol(Symbol::Else)
                .then(|| Expr::parse_block(tokens, exprs.clone(), args));
            return Some(exprs.add(Self::If {
                clause, truthy, falsy,
                span: tokens.span_from(start)
            }));
        }
        if tokens.peek_bracketed(BracketType::Brackets) {
            return Some(Self::parse_block(tokens, exprs.clone(), args));
        }
        if tokens.peek_and_expect_symbol(Symbol::Await) {
            let expr = Expr::parse(tokens, exprs.clone(), args);
            return Some(exprs.add(Self::Await(expr, tokens.span_from(start))));
        }
        if tokens.peek_and_expect_symbol(Symbol::Return) {
            // Don't parse return value if there is a separator or eof coming
            let no_expr = tokens.peek_symbol(Symbol::Semicolon) ||
                tokens.peek_symbol(Symbol::Comma) ||
                tokens.peek().is_none();
            let expr = (!no_expr).then(|| Expr::parse(tokens, exprs.clone(), args));
            return Some(exprs.add(Self::Return(expr, tokens.span_from(start))));
        }

        None
    }
}
