
use crate::{
    ast::expr::{Expr, Parser},
    pools::{exprs::{ExprId}, messages::Message},
    tokens::{token::{Symbol, Token}}
};

impl Expr {
    pub(super) fn parse_semicolon_expr_list(parser: &mut Parser, only_definitions: bool) -> Vec<ExprId> {
        let mut list = Vec::new();
        let parse = if only_definitions { Expr::parse_definition } else { Expr::parse };
        while parser.tokens.peek().is_some() {
            let mut expr = parse(parser);
            let requires_semicolon = parser.exprs.lock().get(expr).requires_semicolon(parser.exprs.clone());

            if requires_semicolon {
                // For error recovery reasons, do not consume unless we 
                // actually got a semicolon
                match parser.tokens.peek() {
                    Some(Token::Symbol(Symbol::Semicolon, _)) => {
                        parser.tokens.next();
                    }
                    tk => {
                        if only_definitions || tk.is_some() {
                            parser.tokens.messages.lock_mut().add(Message::expected(
                                "semicolon",
                                tk.map(|t| t.expected_name()).unwrap_or(parser.tokens.eof_name()),
                                tk.map(|t| t.span()).unwrap_or(parser.tokens.last_span()),
                            ));
                        }
                        // Last statement is transformed into `yield x`
                        else {
                            let span = parser.exprs.lock().get(expr).span();
                            expr = parser.exprs.lock_mut().add(Expr::Yield(expr, span));
                        }
                    }
                }
            }
            list.push(expr);

            // Consume any additional semicolons and warn about them
            let too_many_semicolons_start = parser.tokens.start();
            let mut found_additional_semicolons = false;
            while parser.tokens.peek_and_expect_symbol(Symbol::Semicolon) {
                found_additional_semicolons = true;
            }
            if found_additional_semicolons {
                parser.tokens.messages.lock_mut().add(Message::new_error(
                    "unnecessary semicolon(s)",
                    parser.tokens.span_from(too_many_semicolons_start)
                ));
            }
        }
        list
    }
    pub(super) fn parse_comma(parser: &mut Parser) {
        // For error recovery reasons, do not consume unless we 
        // actually got a comma
        match parser.tokens.peek() {
            Some(Token::Symbol(Symbol::Comma, _)) => {
                parser.tokens.next();
            }
            tk => {
                parser.tokens.messages.lock_mut().add(Message::expected(
                    "comma",
                    tk.map(|t| t.expected_name()).unwrap_or(parser.tokens.eof_name()),
                    tk.map(|t| t.span()).unwrap_or(parser.tokens.last_span()),
                ));
            }
        }
    }
    pub(super) fn parse_comma_list<T>(parse_item: impl Fn(&mut Parser) -> T, parser: &mut Parser) -> Vec<T> {
        let mut items = Vec::new();
        while parser.tokens.peek().is_some() {
            items.push(parse_item(parser));
            // Don't require trailing comma
            if parser.tokens.peek().is_none() {
                break;
            }
            Expr::parse_comma(parser);
        }
        items
    }
}

