use std::fmt::Display;

use crate::{
    ast::expr::Ident,
    pools::{messages::Message, modules::Span},
    codebase::Codebase,
    tokens::token::{BracketType, Symbol, Token},
    utils::lookahead_iter::Looakhead
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
}

impl Tokens {
    pub fn new<S: Display>(tks: Vec<Token>, eof_name: S, first_span: Span) -> Self {
        Self {
            iter: Looakhead::new(tks.into_iter()),
            last_span: first_span,
            eof_name: eof_name.to_string(),
        }
    }
    pub fn peek(&self) -> Option<&Token> {
        self.iter.lookahead(0)
    }
    pub fn peek_n(&self, index: usize) -> Option<&Token> {
        self.iter.lookahead(index)
    }

    fn skip_attributes(&mut self, codebase: &mut Codebase) {
        while self.peek_attr(codebase) {
            let tk = self.next().unwrap();
            codebase.messages.add(Message::new_error("attributes are not allowed here", tk.span()));
        }
    }

    fn peek_no_attrs<F: Fn(&Token) -> bool>(&mut self, codebase: &mut Codebase, matcher: F) -> bool {
        self.skip_attributes(codebase);
        self.peek().is_some_and(matcher)
    }
    fn expect_no_attrs<S: Display, F: Fn(&Token) -> bool>(&mut self, expected: S, codebase: &mut Codebase, matcher: F) -> Token {
        self.skip_attributes(codebase);
        match self.next() {
            Some(t) if matcher(&t) => t,
            Some(bad) => {
                codebase.messages.add(Message::expected(
                    expected.to_string(), bad.expected_name(), bad.span()
                ));
                bad
            }
            None => Token::Int(0, self.last_span)
        }
    }

    pub fn peek_ident(&mut self, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Ident(..)))
    }
    pub fn expect_ident(&mut self, codebase: &mut Codebase) -> Ident {
        if let Token::Ident(name, span) = self.expect_no_attrs(
            "identifier", codebase, |tk| matches!(tk, Token::Ident(..))
        ) {
            return Ident(name, span);
        }
        Ident(codebase.names.missing(), self.last_span)
    }

    pub fn peek_symbol(&mut self, symbol: Symbol, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Symbol(sym, ..) if *sym == symbol))
    }
    pub fn expect_symbol(&mut self, symbol: Symbol, codebase: &mut Codebase) -> Token {
        self.expect_no_attrs(
            format!("'{symbol}'"),
            codebase, 
            |tk| matches!(tk, Token::Symbol(sym, ..) if *sym == symbol)
        )
    }
    pub fn peek_and_expect_symbol(&mut self, symbol: Symbol, codebase: &mut Codebase) -> bool {
        if self.peek_symbol(symbol, codebase) {
            self.next();
            return true;
        }
        false
    }
    pub fn peek_and_expect_symbol_of<F>(&mut self, codebase: &mut Codebase, matches: F) -> Option<(Symbol, Span)>
        where F: Fn(Symbol) -> bool
    {
        if self.peek_no_attrs(codebase, |tk| matches!(tk, Token::Symbol(sym, ..) if matches(*sym))) {
            let Some(Token::Symbol(sym, span)) = self.next() else {
                unreachable!("peek_and_expect_symbol_of: peeked token did not match parsed one");
            };
            return Some((sym, span));
        }
        None
    }

    pub fn peek_int(&mut self, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Int(..)))
    }
    pub fn expect_int(&mut self, codebase: &mut Codebase) -> Token {
        self.expect_no_attrs("integer", codebase, |tk| matches!(tk, Token::Int(..)))
    }
    pub fn peek_float(&mut self, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Float(..)))
    }
    pub fn expect_float(&mut self, codebase: &mut Codebase) -> Token {
        self.expect_no_attrs("float", codebase, |tk| matches!(tk, Token::Float(..)))
    }
    pub fn peek_duration(&mut self, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Duration(..)))
    }
    pub fn expect_duration(&mut self, codebase: &mut Codebase) -> Token {
        self.expect_no_attrs("duration", codebase, |tk| matches!(tk, Token::Duration(..)))
    }
    pub fn peek_str(&mut self, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::String(..)))
    }
    pub fn expect_str(&mut self, codebase: &mut Codebase) -> Token {
        self.expect_no_attrs("string", codebase, |tk| matches!(tk, Token::String(..)))
    }
    pub fn peek_bracketed(&mut self, ty: BracketType, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Bracketed(bty, ..) if *bty == ty))
    }
    pub fn peek_any_bracketed(&mut self, codebase: &mut Codebase) -> bool {
        self.peek_no_attrs(codebase, |p| matches!(p, Token::Bracketed(..)))
    }
    pub fn expect_bracketed(&mut self, ty: BracketType, codebase: &mut Codebase) -> Token {
        self.expect_no_attrs(
            ty.expected_name(),
            codebase, 
            |tk| matches!(tk, Token::Bracketed(bty, ..) if *bty == ty)
        )
    }

    pub fn peek_attr(&mut self, _codebase: &mut Codebase) -> bool {
        // Cannot use peek_no_attrs here since that skips attrs
        self.peek().is_some_and(|p| matches!(p, Token::Attribute(..)))
    }
    pub fn expect_attr(&mut self, codebase: &mut Codebase) -> Token {
        let tk = self.next();
        if !matches!(tk, Some(Token::Attribute(..))) {
            codebase.messages.add(Message::expected(
                "attribute",
                tk.as_ref().map(|t| t.expected_name()).unwrap_or(&self.eof_name),
                tk.as_ref().map(|t| t.span()).unwrap_or(self.last_span)
            ));
        }
        tk.unwrap_or(Token::Int(0, self.last_span))
    }

    pub fn expected(&mut self, what: &str, codebase: &mut Codebase) -> Span {
        // todo: some sort of error recovery? don't parse if there is a separator 
        // (comma or semicolon) here?
        let (name, span) = match self.next() {
            Some(invalid) => (invalid.expected_name(), invalid.span()),
            None => (self.eof_name(), self.last_span()),
        };
        codebase.messages.add(Message::expected(what, name, span));
        span
    }

    pub fn expect_empty(&self, codebase: &mut Codebase) {
        if let Some(p) = self.peek() {
            codebase.messages.add(Message::expected(&self.eof_name, p.expected_name(), p.span()));
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
    use crate::codebase::Codebase;
    
    let (mut codebase, id) = Codebase::new_with_test_package("test_tokenizer", r#"
        let x += -5 + 2.3;
        @thing("dawg", 5.2)
        "Hello, world!\n\t";
    "#);

    let mut tokens = codebase.tokenize_mod(id).unwrap();
    tokens.expect_symbol(Symbol::Let, &mut codebase);
    tokens.expect_ident(&mut codebase);
    tokens.expect_symbol(Symbol::AddAssign, &mut codebase);
    tokens.expect_symbol(Symbol::Minus, &mut codebase);
    tokens.expect_int(&mut codebase);
    tokens.expect_symbol(Symbol::Plus, &mut codebase);
    tokens.expect_float(&mut codebase);
    tokens.expect_symbol(Symbol::Semicolon, &mut codebase);
    
    let Token::Attribute(_, Some(mut sub), _) = tokens.expect_attr(&mut codebase) else {
        panic!();
    };
    sub.expect_str(&mut codebase);
    sub.expect_symbol(Symbol::Comma, &mut codebase);
    sub.expect_float(&mut codebase);
    sub.expect_empty(&mut codebase);

    tokens.expect_str(&mut codebase);
    tokens.expect_symbol(Symbol::Semicolon, &mut codebase);
    tokens.expect_empty(&mut codebase);
    assert!(codebase.messages.count_total() == 0, "{:?}", codebase.messages);
}
