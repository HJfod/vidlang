
use crate::{
    ast::expr::{Expr, ParseArgs},
    pools::{exprs::{ExprId, Exprs}, messages::Message},
    tokens::{token::{Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_semicolon_expr_list(
        tokens: &mut Tokens,
        only_definitions: bool,
        exprs: Exprs,
        args: ParseArgs,
    ) -> Vec<ExprId> {
        let mut list = Vec::new();
        let parse = if only_definitions { Expr::parse_definition } else { Expr::parse };
        while tokens.peek().is_some() {
            let mut expr = parse(tokens, exprs.clone(), args);
            let requires_semicolon = exprs.exec(expr, |e| e.requires_semicolon(exprs.clone()));

            if requires_semicolon {
                // For error recovery reasons, do not consume unless we 
                // actually got a semicolon
                match tokens.peek() {
                    Some(Token::Symbol(Symbol::Semicolon, _)) => {
                        tokens.next();
                    }
                    tk => {
                        if only_definitions || tk.is_some() {
                            tokens.messages().add(Message::expected(
                                "semicolon",
                                tk.map(|t| t.expected_name()).unwrap_or(tokens.eof_name()),
                                tk.map(|t| t.span()).unwrap_or(tokens.last_span()),
                            ));
                        }
                        // Last statement is transformed into `yield x`
                        else {
                            let span = exprs.exec(expr, |e| e.span());
                            expr = exprs.add(Expr::Yield(expr, span));
                        }
                    }
                }
            }
            list.push(expr);

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
        list
    }
    pub(super) fn parse_comma(tokens: &mut Tokens, _exprs: Exprs, _args: ParseArgs) {
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
        exprs: Exprs,
        args: ParseArgs,
    ) -> Vec<T>
        where F: Fn(&mut Tokens, Exprs, ParseArgs) -> T
    {
        let mut items = Vec::new();
        while tokens.peek().is_some() {
            items.push(parse_item(tokens, exprs.clone(), args));
            // Don't require trailing comma
            if tokens.peek().is_none() {
                break;
            }
            Expr::parse_comma(tokens, exprs.clone(), args);
        }
        items
    }
}

