
use crate::{
    ast::expr::{Expr, ParseArgs},
    entities::messages::Message,
    tokens::{token::{Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_semicolon_expr_list(
        tokens: &mut Tokens,
        only_definitions: bool,
        args: ParseArgs,
    ) -> Vec<Expr> {
        let mut exprs = Vec::new();
        let parse = if only_definitions { Expr::parse_definition } else { Expr::parse };
        while tokens.peek().is_some() {
            let mut expr = parse(tokens, args);
            let requires_semicolon = expr.requires_semicolon();

            if requires_semicolon {
                // For error recovery reasons, do not consume unless we 
                // actually got a semicolon
                match tokens.peek() {
                    Some(Token::Symbol(Symbol::Semicolon, _)) => {
                        tokens.next();
                    }
                    tk => {
                        if only_definitions {
                            tokens.messages().add(Message::expected(
                                "semicolon",
                                tk.map(|t| t.expected_name()).unwrap_or(tokens.eof_name()),
                                tk.map(|t| t.span()).unwrap_or(tokens.last_span()),
                            ));
                        }
                        else {
                            let span = expr.span();
                            expr = Expr::Yield(Box::from(expr), span);
                        }
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
    pub(super) fn parse_comma(tokens: &mut Tokens, _args: ParseArgs) {
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
    pub(super) fn parse_comma_list<F, T>(
        parse_item: F,
        tokens: &mut Tokens,
        args: ParseArgs,
    ) -> Vec<T>
        where F: Fn(&mut Tokens, ParseArgs) -> T
    {
        let mut items = Vec::new();
        while tokens.peek().is_some() {
            items.push(parse_item(tokens, args));
            // Don't require trailing comma
            if tokens.peek().is_none() {
                break;
            }
            Expr::parse_comma(tokens, args);
        }
        items
    }
}

