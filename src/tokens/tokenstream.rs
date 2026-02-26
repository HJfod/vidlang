use std::fmt::Display;

use crate::{entities::{codebase::Span, messages::{Message, Messages}, names::{MISSING_NAME, Names}}, tokens::token::{BracketType, Symbol, Token}};
use concat_idents::concat_idents;

// Realistically most of our file is already in one big Vec<Token> anyway because 
// of brackets containing subtrees so might as well make it all a big Vec<Token>
#[derive(Debug)]
pub struct Tokens {
    iter: std::vec::IntoIter<Token>,
    peeked: Option<Token>,
    last_span: Span,
    eof_name: String,
    names: Names,
    messages: Messages,
}

impl Tokens {
    pub fn new<S: Display>(tks: Vec<Token>, eof_name: S, first_span: Span, names: Names, messages: Messages) -> Self {
        let mut iter = tks.into_iter();
        Self {
            peeked: iter.next(),
            iter,
            last_span: first_span,
            eof_name: eof_name.to_string(),
            names,
            messages,
        }
    }
    pub fn peek(&self) -> Option<&Token> {
        self.peeked.as_ref()
    }

    pub fn peek_symbol(&self, symbol: Symbol) -> bool {
        self.peeked.as_ref().is_some_and(|p| match p {
            Token::Symbol(sym, _) => *sym == symbol,
            _ => false,
        })
    }
    pub fn expect_symbol(&mut self, symbol: Symbol) -> Token {
        let tk = self.next();
        if tk.as_ref().is_none_or(|tk| match tk {
            Token::Symbol(sym, _) => *sym != symbol,
            _ => true,
        }) {
            self.messages.add(Message::expected(
                format!("'{symbol}'"),
                tk.as_ref().map(|t| t.expected_name()).unwrap_or(&self.eof_name),
                tk.as_ref().map(|t| t.span()).unwrap_or(self.last_span)
            ));
        }
        tk.unwrap_or(Token::Symbol(symbol, self.last_span))
    }
    pub fn peek_and_expect_symbol(&mut self, symbol: Symbol) -> bool {
        if self.peek_symbol(symbol) {
            self.next();
            return true;
        }
        false
    }
    pub fn peek_and_expect_symbol_of<F>(&mut self, matches: F) -> Option<(Symbol, Span)>
        where F: Fn(Symbol) -> bool
    {
        if self.peeked.as_ref().is_some_and(|p| match p {
            Token::Symbol(sym, _) => matches(*sym),
            _ => false,
        }) {
            let Some(Token::Symbol(sym, span)) = self.next() else {
                unreachable!("peek_and_expect_symbol_of: peeked token did not match parsed one");
            };
            return Some((sym, span));
        }
        None
    }

    pub fn peek_bracketed(&self, ty: BracketType) -> bool {
        self.peeked.as_ref().is_some_and(|p| match p {
            Token::Bracketed(bty, _, _) => *bty == ty,
            _ => false,
        })
    }
    pub fn expect_bracketed(&mut self, ty: BracketType) -> Token {
        let tk = self.next();
        if tk.as_ref().is_none_or(|tk| match tk {
            Token::Bracketed(bty, _, _) => *bty != ty,
            _ => true,
        }) {
            self.messages.add(Message::expected(
                ty.expected_name(),
                tk.as_ref().map(|t| t.expected_name()).unwrap_or(&self.eof_name),
                tk.as_ref().map(|t| t.span()).unwrap_or(self.last_span)
            ));
        }
        tk.unwrap_or_else(|| Token::Bracketed(
            ty,
            Box::from(Tokens::new(
                vec![],
                ty.close().to_string(),
                self.last_span,
                self.names(),
                self.messages()
            )),
            self.last_span
        ))
    }

    pub fn expected(&mut self, what: &str) -> Span {
        // todo: some sort of error recovery? don't parse if there is a separator 
        // (comma or semicolon) here?
        let (name, span) = match self.next() {
            Some(invalid) => (invalid.expected_name(), invalid.span()),
            None => (self.eof_name(), self.last_span()),
        };
        self.messages.add(Message::expected(what, name, span));
        span
    }

    pub fn expect_empty(&self) {
        if let Some(ref p) = self.peeked {
            self.messages.add(Message::expected(&self.eof_name, p.expected_name(), p.span()));
        }
    }

    pub fn start(&self) -> usize {
        self.peeked.as_ref().map(|p| p.span()).unwrap_or(self.last_span).start()
    }
    pub fn span_from(&self, start: usize) -> Span {
        self.last_span.extend_from(start)
    }
    pub fn last_span(&self) -> Span {
        self.last_span
    }
    pub fn eof_name(&self) -> &str {
        &self.eof_name
    }
    pub fn names(&self) -> Names {
        self.names.clone()
    }
    pub fn messages(&self) -> Messages {
        self.messages.clone()
    }
}

macro_rules! impl_tokens_expect {
    ($name: ident, $variant: ident, $default_val: expr) => {
        impl_tokens_expect!(
            $name,
            Token::$variant(_, _),
            |span| Token::$variant($default_val, span)
        );
    };
    ($name: ident, $pat: pat, $default_pat: expr) => {
        concat_idents!(__peek_name = peek_, $name {
            impl Tokens {
                pub fn __peek_name(&self) -> bool {
                    matches!(self.peeked, Some($pat))
                }
            }
        });
        concat_idents!(__expect_name = expect_, $name {
            impl Tokens {
                pub fn __expect_name(&mut self) -> Token {
                    match self.next() {
                        Some(tk @ Token::Int(_, _)) => tk,
                        Some(wrong_tk) => {
                            let span = wrong_tk.span();
                            let right_tk = ($default_pat)(span);
                            self.messages.add(Message::expected(
                                right_tk.expected_name(), wrong_tk.expected_name(), span
                            ));
                            right_tk
                        }
                        None => {
                            let span = self.last_span;
                            let right_tk = ($default_pat)(span);
                            self.messages.add(Message::expected(
                                right_tk.expected_name(), "eof", span
                            ));
                            right_tk
                        }
                    }
                }
            }
        });
    };
}

impl_tokens_expect!(int, Int, 0);
impl_tokens_expect!(float, Float, 0.0);
impl_tokens_expect!(str, String, vec![]);
impl_tokens_expect!(ident, Ident, MISSING_NAME);
impl_tokens_expect!(attr, Token::Attribute(_, _, _), |span| Token::Attribute(MISSING_NAME, None, span));

impl Iterator for Tokens {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        match std::mem::replace(&mut self.peeked, self.iter.next()) {
            Some(tk) => {
                self.last_span = tk.span().next_ch();
                Some(tk)
            }
            None => None,
        }
    }
}
