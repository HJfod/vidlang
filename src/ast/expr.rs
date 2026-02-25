use crate::{ast::{token::{StrLitComp, Token}, tokenizer::Tokens}, entities::{messages::Messages, names::NameId, src::Span}};

pub enum StringComp {
    String(String),
    Expr(Expr),
}

pub enum Expr {
    Int(u64, Span),
    Float(f64, Span),
    String(Vec<StringComp>, Span),
    Ident(NameId, Span),
}

impl Expr {
    fn parse_base(tokens: &mut Tokens, messages: Messages) -> Self {
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
        todo!()
    }
    pub fn parse(tokens: &mut Tokens, messages: Messages) -> Self {
        todo!()
    }
}
