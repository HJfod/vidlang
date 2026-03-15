
use crate::{
    ast::expr::{Expr, ParseArgs},
    pools::exprs::ExprId,
    codebase::Codebase,
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_block(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let tk = tokens.expect_bracketed(BracketType::Braces, codebase);
        if let Token::Bracketed(_, mut content, span) = tk {
            let list = Expr::parse_semicolon_expr_list(&mut content, codebase, false, args);
            codebase.exprs.add(Self::Block(list, span))
        }
        else {
            codebase.exprs.add(Self::Ident(codebase.names.missing_path(tk.span())))
        }
    }
    pub(super) fn try_parse_control_flow(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs)
     -> Option<ExprId>
    {
        let start = tokens.start();

        if tokens.peek_and_expect_symbol(Symbol::If, codebase) {
            let clause = Expr::parse_expr(tokens, codebase, args);
            let truthy = Expr::parse_block(tokens, codebase, args);
            let falsy = tokens.peek_and_expect_symbol(Symbol::Else, codebase)
                .then(|| Expr::parse_block(tokens, codebase, args));
            return Some(codebase.exprs.add(Self::If {
                clause, truthy, falsy,
                span: tokens.span_from(start)
            }));
        }
        if tokens.peek_bracketed(BracketType::Braces, codebase) {
            return Some(Self::parse_block(tokens, codebase, args));
        }
        if tokens.peek_and_expect_symbol(Symbol::Await, codebase) {
            let expr = Expr::parse_expr(tokens, codebase, args);
            return Some(codebase.exprs.add(Self::Await(expr, tokens.span_from(start))));
        }
        if tokens.peek_and_expect_symbol(Symbol::Return, codebase) {
            // Don't parse return value if there is a separator or eof coming
            let no_expr = tokens.peek_symbol(Symbol::Semicolon, codebase) ||
                tokens.peek_symbol(Symbol::Comma, codebase) ||
                tokens.peek().is_none();
            let expr = (!no_expr).then(|| Expr::parse_expr(tokens, codebase, args));
            return Some(codebase.exprs.add(Self::Return(expr, tokens.span_from(start))));
        }

        None
    }
}
