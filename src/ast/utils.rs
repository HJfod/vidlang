
use crate::{
    ast::expr::{Expr, ParseArgs},
    pools::{exprs::ExprId, messages::Message},
    codebase::Codebase,
    tokens::{token::{Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_semicolon_expr_list(
        tokens: &mut Tokens,
        codebase: &mut Codebase,
        only_definitions: bool,
        args: ParseArgs,
    ) -> Vec<ExprId> {
        let mut list = Vec::new();
        let parse = if only_definitions { Expr::parse_definition } else { Expr::parse };
        while tokens.peek().is_some() {
            let mut expr = parse(tokens, codebase, args);
            let requires_semicolon = codebase.exprs.get(expr).requires_semicolon(codebase);

            if requires_semicolon {
                // For error recovery reasons, do not consume unless we 
                // actually got a semicolon
                match tokens.peek() {
                    Some(Token::Symbol(Symbol::Semicolon, _)) => {
                        tokens.next();
                    }
                    tk => {
                        if only_definitions || tk.is_some() {
                            codebase.messages.add(Message::expected(
                                "semicolon",
                                tk.map(|t| t.expected_name()).unwrap_or(tokens.eof_name()),
                                tk.map(|t| t.span()).unwrap_or(tokens.last_span()),
                            ));
                        }
                        // Last statement is transformed into `yield x`
                        else {
                            let span = codebase.exprs.get(expr).span();
                            expr = codebase.exprs.add(Expr::Yield(expr, span));
                        }
                    }
                }
            }
            list.push(expr);

            // Consume any additional semicolons and warn about them
            let too_many_semicolons_start = tokens.start();
            let mut found_additional_semicolons = false;
            while tokens.peek_and_expect_symbol(Symbol::Semicolon, codebase) {
                found_additional_semicolons = true;
            }
            if found_additional_semicolons {
                codebase.messages.add(Message::new_error(
                    "unnecessary semicolon(s)",
                    tokens.span_from(too_many_semicolons_start)
                ));
            }
        }
        list
    }
    pub(super) fn parse_comma(tokens: &mut Tokens, codebase: &mut Codebase, _args: ParseArgs) {
        // For error recovery reasons, do not consume unless we 
        // actually got a comma
        match tokens.peek() {
            Some(Token::Symbol(Symbol::Comma, _)) => {
                tokens.next();
            }
            tk => {
                codebase.messages.add(Message::expected(
                    "comma",
                    tk.map(|t| t.expected_name()).unwrap_or(tokens.eof_name()),
                    tk.map(|t| t.span()).unwrap_or(tokens.last_span()),
                ));
            }
        }
    }
    pub(super) fn parse_comma_list<T>(
        tokens: &mut Tokens,
        codebase: &mut Codebase,
        args: ParseArgs,
        parse_item: impl Fn(&mut Tokens, &mut Codebase, ParseArgs) -> T
    ) -> Vec<T> {
        let mut items = Vec::new();
        while tokens.peek().is_some() {
            items.push(parse_item(tokens, codebase, args));
            // Don't require trailing comma
            if tokens.peek().is_none() {
                break;
            }
            Expr::parse_comma(tokens, codebase, args);
        }
        items
    }
}

