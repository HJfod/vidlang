use crate::{
    ast::expr::{Expr, Ident, StringComp},
    entities::{messages::Messages, names::MISSING_NAME},
    tokens::{token::{StrLitComp, Token},
    tokenstream::Tokens
}};

impl Expr {
    pub(super) fn parse_name(tokens: &mut Tokens, messages: Messages) -> Ident {
        if tokens.peek_ident() {
            let Token::Ident(name, span) = tokens.expect_ident(messages.clone()) else {
                unreachable!("tokens.peek_ident() returned true but expect_ident() did not return an ident");
            };
            return Ident(name, span);
        }
        let span = tokens.expected("identifier", messages);
        Ident(MISSING_NAME, span)
    }
    pub(super) fn parse_literal(tokens: &mut Tokens, messages: Messages) -> Expr {
        if tokens.peek_int() {
            let Token::Int(num, span) = tokens.expect_int(messages.clone()) else {
                unreachable!("tokens.peek_int() returned true but expect_int() did not return an integer");
            };
            return Expr::Int(num, span);
        }
        if tokens.peek_float() {
            let Token::Float(num, span) = tokens.expect_float(messages.clone()) else {
                unreachable!("tokens.peek_float() returned true but expect_float() did not return a float");
            };
            return Expr::Float(num, span);
        }
        if tokens.peek_str() {
            let Token::String(value, span) = tokens.expect_str(messages.clone()) else {
                unreachable!("tokens.peek_str() returned true but expect_str() did not return a string");
            };
            return Expr::String(
                value.into_iter().map(|c| match c {
                    StrLitComp::String(s) => StringComp::String(s),
                    StrLitComp::Component(mut c) => {
                        let expr = Expr::parse(&mut c, messages.clone());
                        c.expect_empty(messages.clone());
                        StringComp::Expr(expr)
                    }
                }).collect(),
                span
            );
        }
        let span = tokens.expected("expression", messages);
        Expr::Ident(Ident(MISSING_NAME, span))
    }
}
