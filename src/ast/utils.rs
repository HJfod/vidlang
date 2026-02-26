
use crate::{
    ast::expr::Expr,
    entities::{messages::Message},
    tokens::{token::{Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_semicolon_list<F>(parse_item: F, tokens: &mut Tokens) -> Vec<Expr>
        where F: Fn(&mut Tokens) -> Expr
    {
        let mut exprs = Vec::new();
        while tokens.peek().is_some() {
            let expr = parse_item(tokens);
            if expr.requires_semicolon() {
                // For error recovery reasons, do not consume unless we 
                // actually got a semicolon
                match tokens.peek() {
                    Some(Token::Symbol(Symbol::Semicolon, _)) => {
                        tokens.next();
                    }
                    tk => {
                        tokens.messages().add(Message::expected(
                            "semicolon",
                            tk.map(|t| t.expected_name()).unwrap_or(tokens.eof_name()),
                            tk.map(|t| t.span()).unwrap_or(tokens.last_span()),
                        ));
                    }
                }
            }
            exprs.push(expr);
        }
        exprs
    }
}

