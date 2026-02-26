
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
            let requires_semicolon = expr.requires_semicolon();

            if requires_semicolon {
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

            // Consume any additional semicolons and warn about them
            let too_many_semicolons_start = tokens.start();
            let mut found_additional_semicolons = false;
            while tokens.peek_and_expect_symbol(Symbol::Semicolon) {
                found_additional_semicolons = true;
            }
            if found_additional_semicolons {
                tokens.messages().add(Message::new_error(
                    "unnecessary semicolon(s)",
                    tokens.span_from(too_many_semicolons_start)
                ));
            }
        }
        exprs
    }
    pub(super) fn parse_comma_list<F, T>(parse_item: F, tokens: &mut Tokens) -> Vec<T>
        where F: Fn(&mut Tokens) -> T
    {
        let mut items = Vec::new();
        while tokens.peek().is_some() {
            items.push(parse_item(tokens));

            // Don't require trailing comma
            if tokens.peek().is_none() {
                break;
            }

            // For error recovery reasons, do not consume unless we 
            // actually got a comma
            match tokens.peek() {
                Some(Token::Symbol(Symbol::Comma, _)) => {
                    tokens.next();
                }
                tk => {
                    tokens.messages().add(Message::expected(
                        "comma",
                        tk.map(|t| t.expected_name()).unwrap_or(tokens.eof_name()),
                        tk.map(|t| t.span()).unwrap_or(tokens.last_span()),
                    ));
                }
            }
        }
        items
    }
}

