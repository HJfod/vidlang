use crate::{
    ast::expr::{Expr, Ident, ParseArgs, StringComp},
    entities::names::MISSING_NAME,
    tokens::{token::{BracketType, StrLitComp, Token},
    tokenstream::Tokens
}};

impl Expr {
    pub(super) fn parse_value(tokens: &mut Tokens, args: ParseArgs) -> Expr {
        if tokens.peek_int() {
            let Token::Int(num, span) = tokens.expect_int() else {
                unreachable!("tokens.peek_int() returned true but expect_int() did not return an integer");
            };
            return Expr::Int(num, span);
        }
        if tokens.peek_float() {
            let Token::Float(num, span) = tokens.expect_float() else {
                unreachable!("tokens.peek_float() returned true but expect_float() did not return a float");
            };
            return Expr::Float(num, span);
        }
        if tokens.peek_str() {
            let Token::String(value, span) = tokens.expect_str() else {
                unreachable!("tokens.peek_str() returned true but expect_str() did not return a string");
            };
            return Expr::String(
                value.into_iter().map(|c| match c {
                    StrLitComp::String(s) => StringComp::String(s),
                    StrLitComp::Component(mut c) => {
                        let expr = Expr::parse(&mut c, args);
                        c.expect_empty();
                        StringComp::Expr(expr)
                    }
                }).collect(),
                span
            );
        }
        if tokens.peek_ident() {
            return Expr::Ident(tokens.expect_ident());
        }
        if tokens.peek_bracketed(BracketType::Parentheses) {
            let start = tokens.start();
            let Token::Bracketed(_, mut sub_tokens, _) = tokens.expect_bracketed(BracketType::Parentheses) else {
                unreachable!("tokens.expect_bracketed didnt return Bracketed despite being peeked");
            };
            let fields = Expr::parse_comma_list(Expr::parse, &mut sub_tokens, args);
            return Expr::Tuple(fields, tokens.span_from(start));
        }
        let span = tokens.expected("expression");
        Expr::Ident(Ident(MISSING_NAME, span))
    }
}
