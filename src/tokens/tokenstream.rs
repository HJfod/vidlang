use std::fmt::Display;

use crate::{
    ast::expr::Ident, pools::{PoolRef, codebase::Span, messages::{Message, Messages}, names::Names}, tokens::token::{BracketType, Symbol, Token}, utils::lookahead_iter::Looakhead
};

// Realistically most of our file is already in one big Vec<Token> anyway because 
// of brackets containing subtrees so might as well make it all a big Vec<Token>
#[derive(Debug)]
pub struct Tokens {
    // Some constructs (arrow functions, named call args) require two tokens 
    // of lookahead to parse without backtracking
    iter: Looakhead<std::vec::IntoIter<Token>, 2>,
    last_span: Span,
    eof_name: String,
    pub names: PoolRef<Names>,
    pub messages: PoolRef<Messages>,
}

impl Tokens {
    pub fn new<S: Display>(tks: Vec<Token>, eof_name: S, first_span: Span, names: PoolRef<Names>, messages: PoolRef<Messages>) -> Self {
        Self {
            iter: Looakhead::new(tks.into_iter()),
            last_span: first_span,
            eof_name: eof_name.to_string(),
            names,
            messages,
        }
    }
    pub fn peek(&self) -> Option<&Token> {
        self.iter.lookahead(0)
    }
    pub fn peek_n(&self, index: usize) -> Option<&Token> {
        self.iter.lookahead(index)
    }

    fn skip_attributes(&mut self) {
        while self.peek_attr() {
            let tk = self.next().unwrap();
            self.messages.lock_mut().add(Message::new_error("attributes are not allowed here", tk.span()));
        }
    }

    fn peek_no_attrs<F: Fn(&Token) -> bool>(&mut self, matcher: F) -> bool {
        self.skip_attributes();
        self.peek().is_some_and(matcher)
    }
    fn expect_no_attrs<S: Display, F: Fn(&Token) -> bool>(&mut self, expected: S, matcher: F) -> Token {
        self.skip_attributes();
        match self.next() {
            Some(t) if matcher(&t) => t,
            Some(bad) => {
                self.messages.lock_mut().add(Message::expected(
                    expected.to_string(), bad.expected_name(), bad.span()
                ));
                bad
            }
            None => Token::Int(0, self.last_span)
        }
    }

    pub fn peek_ident(&mut self) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::Ident(..)))
    }
    pub fn expect_ident(&mut self) -> Ident {
        if let Token::Ident(name, span) = self.expect_no_attrs(
            "identifier", |tk| matches!(tk, Token::Ident(..))
        ) {
            return Ident(name, span);
        }
        Ident(self.names.lock_mut().missing(), self.last_span)
    }

    pub fn peek_symbol(&mut self, symbol: Symbol) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::Symbol(sym, ..) if *sym == symbol))
    }
    pub fn expect_symbol(&mut self, symbol: Symbol) -> Token {
        self.expect_no_attrs(
            format!("'{symbol}'"),
            |tk| matches!(tk, Token::Symbol(sym, ..) if *sym == symbol)
        )
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
        if self.peek_no_attrs(|tk| matches!(tk, Token::Symbol(sym, ..) if matches(*sym))) {
            let Some(Token::Symbol(sym, span)) = self.next() else {
                unreachable!("peek_and_expect_symbol_of: peeked token did not match parsed one");
            };
            return Some((sym, span));
        }
        None
    }

    pub fn peek_int(&mut self) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::Int(..)))
    }
    pub fn expect_int(&mut self) -> Token {
        self.expect_no_attrs("integer", |tk| matches!(tk, Token::Int(..)))
    }
    pub fn peek_float(&mut self) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::Float(..)))
    }
    pub fn expect_float(&mut self) -> Token {
        self.expect_no_attrs("float", |tk| matches!(tk, Token::Float(..)))
    }
    pub fn peek_duration(&mut self) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::Duration(..)))
    }
    pub fn expect_duration(&mut self) -> Token {
        self.expect_no_attrs("duration", |tk| matches!(tk, Token::Duration(..)))
    }
    pub fn peek_str(&mut self) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::String(..)))
    }
    pub fn expect_str(&mut self) -> Token {
        self.expect_no_attrs("string", |tk| matches!(tk, Token::String(..)))
    }
    pub fn peek_bracketed(&mut self, ty: BracketType) -> bool {
        self.peek_no_attrs(|p| matches!(p, Token::Bracketed(bty, ..) if *bty == ty))
    }
    pub fn expect_bracketed(&mut self, ty: BracketType) -> Token {
        self.expect_no_attrs(
            ty.expected_name(),
            |tk| matches!(tk, Token::Bracketed(bty, ..) if *bty == ty)
        )
    }

    pub fn peek_attr(&mut self) -> bool {
        // Cannot use peek_no_attrs here since that skips attrs
        self.peek().is_some_and(|p| matches!(p, Token::Attribute(..)))
    }
    pub fn expect_attr(&mut self) -> Token {
        let tk = self.next();
        if !matches!(tk, Some(Token::Attribute(..))) {
            self.messages.lock_mut().add(Message::expected(
                "attribute",
                tk.as_ref().map(|t| t.expected_name()).unwrap_or(&self.eof_name),
                tk.as_ref().map(|t| t.span()).unwrap_or(self.last_span)
            ));
        }
        tk.unwrap_or(Token::Int(0, self.last_span))
    }

    pub fn expected(&mut self, what: &str) -> Span {
        // todo: some sort of error recovery? don't parse if there is a separator 
        // (comma or semicolon) here?
        let (name, span) = match self.next() {
            Some(invalid) => (invalid.expected_name(), invalid.span()),
            None => (self.eof_name(), self.last_span()),
        };
        self.messages.lock_mut().add(Message::expected(what, name, span));
        span
    }

    pub fn expect_empty(&self) {
        if let Some(p) = self.peek() {
            self.messages.lock_mut().add(Message::expected(&self.eof_name, p.expected_name(), p.span()));
        }
    }

    pub fn start(&self) -> usize {
        self.peek().map(|p| p.span()).unwrap_or(self.last_span).start()
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
}

impl Iterator for Tokens {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(tk) => {
                self.last_span = tk.span().next_ch();
                Some(tk)
            }
            None => None,
        }
    }
}

#[test]
fn tokenizing() {
    use crate::pools::codebase::Codebase;
    
    let names = Names::new();
    let messages = Messages::new();
    let (codebase, id) = Codebase::new_with_test_package("test_tokenizer", r#"
        let x += -5 + 2.3;
        @thing("dawg", 5.2)
        "Hello, world!\n\t";
    "#);

    let mut tokens = codebase.tokenize(id, names, messages.clone()).unwrap();
    tokens.expect_symbol(Symbol::Let);
    tokens.expect_ident();
    tokens.expect_symbol(Symbol::AddAssign);
    tokens.expect_symbol(Symbol::Minus);
    tokens.expect_int();
    tokens.expect_symbol(Symbol::Plus);
    tokens.expect_float();
    tokens.expect_symbol(Symbol::Semicolon);
    
    let Token::Attribute(_, Some(mut sub), _) = tokens.expect_attr() else {
        panic!();
    };
    sub.expect_str();
    sub.expect_symbol(Symbol::Comma);
    sub.expect_float();
    sub.expect_empty();

    tokens.expect_str();
    tokens.expect_symbol(Symbol::Semicolon);
    tokens.expect_empty();
    assert!(messages.lock().count_total() == 0, "{messages:?}");
}
