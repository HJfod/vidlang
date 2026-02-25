use std::{str::FromStr, sync::Arc};

use crate::entities::{
    messages::{Message, Messages},
    names::{NameId, Names},
    src::{Span, SrcIterator}
};
use unicode_xid::UnicodeXID;

#[derive(Debug, Clone, Copy)]
pub enum BracketType {
    Parentheses,
    Brackets,
    Braces,
    AngleBrackets,
}

impl BracketType {
    pub fn from_open(ch: char) -> Option<BracketType> {
        match ch {
            '(' => Some(BracketType::Parentheses),
            '[' => Some(BracketType::Brackets),
            '{' => Some(BracketType::Braces),
            '<' => Some(BracketType::AngleBrackets),
            _   => None,
        }
    }
    pub fn open(&self) -> char {
        match self {
            BracketType::Parentheses   => '(',
            BracketType::Brackets      => '[',
            BracketType::Braces        => '{',
            BracketType::AngleBrackets => '<',
        }
    }
    pub fn close(&self) -> char {
        match self {
            BracketType::Parentheses   => ')',
            BracketType::Brackets      => ']',
            BracketType::Braces        => '}',
            BracketType::AngleBrackets => '>',
        }
    }
}

#[derive(Debug, strum_macros::Display, strum_macros::EnumString, PartialEq, Eq)]
#[strum(serialize_all="lowercase")]
pub enum Symbol {
    // Keywords
    Let, Const, Type, Function,
    Trait, Impl, Struct, Clip,
    // Operators
    #[strum(to_string="=")]
    Assign,
    #[strum(to_string="+=")]
    SumAssign,
    #[strum(to_string="-=")]
    SubAssign,
    #[strum(to_string="==")]
    Eq,
    #[strum(to_string=".")]
    Dot,
    #[strum(to_string="...")]
    DotDotDot,
    #[strum(to_string=";")]
    Semicolon,
}

#[derive(Debug)]
pub enum StrLitComp {
    String(String),
    Component(Tokens),
}

#[derive(Debug)]
pub enum Token {
    // Integer literals don't take into account the '-' ever, so we can parse 
    // into an u64 instead for more precision
    Int(u64, Span),
    Float(f64, Span),
    String(Vec<StrLitComp>, Span),
    Ident(NameId, Span),
    Symbol(Symbol, Span),
    Bracketed(BracketType, Box<Tokens>, Span),
    Attribute(NameId, Option<Box<Tokens>>),
}

pub struct Tokenizer<'s> {
    iter: &'s mut SrcIterator<'s>,
    names: Names,
    messages: Messages,
}

impl<'s> Tokenizer<'s> {
    pub fn new(iter: &'s mut SrcIterator<'s>, names: Names, messages: Messages) -> Self {
        Self { iter, names, messages }
    }
    fn skip_to_next(&mut self) {
        while let Some(c) = self.iter.peek() {
            if c.is_whitespace() {
                self.iter.next();
            }
            // Comments (no doc comments)
            else if c == '/' && self.iter.peek_n(1) == Some('/') && self.iter.peek_n(2) != Some('/') {
                // Skip everything until end of line
                for n in &mut *self.iter {
                    if n == '\n' {
                        break;
                    }
                }
            }
            else {
                break;
            }
        }
    }
}

impl<'s> Iterator for Tokenizer<'s> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.skip_to_next();
        let start = self.iter.index();
        let c = self.iter.next()?;

        // String literals
        if c == '"' {
            let mut comps = Vec::new();
            'parse_comps: loop {
                let Some(c) = self.iter.next() else {
                    self.messages.add(Message::expected(
                        "closing quote", "eof", self.iter.head()
                    ));
                    break 'parse_comps;
                };
                // Closing quote
                if c == '"' {
                    break 'parse_comps;
                }
                // Interpolated expressions like `"2 + 2 = {2 + 2}"`
                if c == '{' {
                    let mut tokens = Vec::new();
                    'parse_expr: loop {
                        self.skip_to_next();
                        if self.iter.peek() == Some('}') {
                            self.iter.next();
                            break 'parse_expr;
                        }
                        // No need to check for EOF since this checks it for us
                        let Some(tk) = self.next() else {
                            self.messages.add(Message::expected(
                                "closing brace", "eof", self.iter.head()
                            ));
                            break 'parse_comps;
                        };
                        tokens.push(tk);
                    }
                    comps.push(StrLitComp::Component(Tokens::new(tokens)))
                }
                // Otherwise push the char (or escaped char)
                else {
                    let char_start = self.iter.index();
                    let char_to_push = match c {
                        '\\' => {
                            let Some(escaped) = self.iter.next() else {
                                self.messages.add(Message::expected(
                                    "escaped character", "eof", self.iter.head()
                                ));
                                break 'parse_comps;
                            };
                            match escaped {
                                '\\' => '\\',
                                '\'' => '\'',
                                '"'  => '"',
                                'n'  => '\n',
                                'r'  => '\r',
                                't'  => '\t',
                                '{'  => '{',
                                '}'  => '}',
                                c => {
                                    self.messages.add(Message::new_error(
                                        format!("unknown escape sequence \"\\{c}\""),
                                        self.iter.span_from(char_start)
                                    ));
                                    c
                                }
                            }
                        }
                        c => c
                    };
                    if comps.last().is_none_or(|c| matches!(c, StrLitComp::String(_))) {
                        comps.push(StrLitComp::String(String::new()));
                    }
                    // SAFETY: The line above should ensure that the last 
                    // component is a string
                    let Some(StrLitComp::String(s)) = comps.last_mut() else {
                        unreachable!("Last component was not StrLitComp::String");
                    };
                    s.push(char_to_push);
                }
            }
            return Some(Token::String(comps, self.iter.span_from(start)))
        }

        // Number literals
        if c.is_ascii_digit() {
            let mut num_str = String::from(c) + &self.iter.next_while(|c| c.is_ascii_digit());
            // Floating point number
            if self.iter.peek() == Some('.') {
                num_str.push(self.iter.next().unwrap());
                num_str.push_str(&self.iter.next_while(|c| c.is_ascii_digit()));
                let span = self.iter.span_from(start);
                match num_str.parse::<f64>() {
                    Ok(v) => return Some(Token::Float(v, span)),
                    Err(e) => {
                        self.messages.add(Message::new_error(
                            format!("invalid float literal: {e}"),
                            span
                        ));
                    }
                }
            }
            // Otherwise this is an integer
            let span = self.iter.span_from(start);
            match num_str.parse::<u64>() {
                Ok(v) => return Some(Token::Int(v, span)),
                Err(e) => {
                    self.messages.add(Message::new_error(
                        format!("invalid integer literal: {e}"),
                        span
                    ));
                }
            }
        }

        // Identifiers & keywords
        // Note that '_' is not XID_Start
        if c.is_xid_start() || c == '_' {
            let ident = String::from(c) + &self.iter.next_while(UnicodeXID::is_xid_continue);
            let span = self.iter.span_from(start);
            return Some(match Symbol::from_str(&ident) {
                Ok(sym) => Token::Symbol(sym, span),
                Err(_) => Token::Ident(self.names.add(&ident), span),
            });
        }

        // Brackets
        if let Some(ty) = BracketType::from_open(c) {
            let mut contents = Vec::new();
            loop {
                self.skip_to_next();
                if self.iter.peek() == Some(ty.close()) {
                    self.iter.next();
                    break;
                }
                let Some(tk) = self.next() else {
                    self.messages.add(Message::expected(
                        format!("'{}'", ty.close()), "eof", self.iter.head()
                    ));
                    break;
                };
                contents.push(tk);
            }
            return Some(Token::Bracketed(
                ty, Box::from(Tokens::new(contents)), self.iter.span_from(start)
            ));
        }

        // Attributes
        if c == '@' {
            let ident = if self.iter.peek().is_some_and(|c| c.is_xid_start()) {
                self.iter.next();
                String::from(c) + &self.iter.next_while(UnicodeXID::is_xid_continue)
            }
            else {
                self.messages.add(Message::expected_what(
                    "identifier for attribute", self.iter.head()
                ));
                String::from("<unnamed>")
            };
            // Args for attributes
            let args = self.iter.peek().is_some_and(|c| c == '(').then(|| {
                let Some(Token::Bracketed(_, contents, _)) = self.next() else {
                    unreachable!("Token that starts with '(' was not Token::Bracketed");
                };
                contents
            });
            return Some(Token::Attribute(self.names.add(&ident), args));
        }

        // Otherwise it must be an operator or an invalid character
        let mut sym = String::from(c);

        #[allow(clippy::if_same_then_else)]
        // Everything like +=, -=, etc.
        if "=:+-*/%^~|<>?!".contains(c) && self.iter.peek() == Some('=') {
            sym.push(self.iter.next().unwrap());
        }
        // **
        else if c == '*' && self.iter.peek() == Some('*') {
            sym.push(self.iter.next().unwrap());
        }
        // ??
        else if c == '?' && self.iter.peek() == Some('?') {
            sym.push(self.iter.next().unwrap());
        }
        // All of the dots
        else if c == '.' {
            while self.iter.peek() == Some('.') {
                sym.push(self.iter.next().unwrap());
            }
        }

        match Symbol::from_str(&sym) {
            Ok(sym) => Some(Token::Symbol(sym, self.iter.span_from(start))),
            // Simply skip invalid characters
            Err(_) => {
                self.messages.add(Message::new_error(
                    format!("invalid symbol '{}'", sym),
                    self.iter.span_from(start)
                ));
                self.next()
            }
        }
    }
}

// Realistically most of our file is already in one big Vec<Token> anyway because 
// of brackets containing subtrees so might as well make it all a big Vec<Token>
#[derive(Debug)]
pub struct Tokens {
    iter: std::vec::IntoIter<Token>,
    peeked: Option<Token>,
}

impl Tokens {
    pub fn new(tks: Vec<Token>) -> Self {
        let mut iter = tks.into_iter();
        Self {
            peeked: iter.next(),
            iter,
        }
    }
    pub fn peek(&self) -> Option<&Token> {
        self.peeked.as_ref()
    }
}

impl Iterator for Tokens {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        std::mem::replace(&mut self.peeked, self.iter.next())
    }
}

#[test]
fn test_symbols() {
    assert_eq!(Symbol::from_str("+="), Ok(Symbol::SumAssign));
    assert_eq!(Symbol::from_str("let"), Ok(Symbol::Let));
    assert!(Symbol::from_str("++=").is_err());
}

#[test]
fn test_tokenizer() {
    use crate::entities::src::Codebase;
    let names = Names::new();
    let messages = Messages::new();
    let mut codebase = Codebase::new();
    let id = codebase.add_memory("test_tokenizer", r#"
        let x += 5;
    "#);
    let tokens = codebase.tokenize(id, names, messages.clone()).collect::<Vec<_>>();
    assert!(messages.count_total() == 0, "{messages:?}");
    assert_eq!(tokens.len(), 5);
    assert!(matches!(tokens[0], Token::Symbol(Symbol::Let, _)));
    assert!(matches!(tokens[1], Token::Ident(_, _)));
    assert!(matches!(tokens[2], Token::Symbol(Symbol::SumAssign, _)));
    assert!(matches!(tokens[3], Token::Int(_, _)));
    assert!(matches!(tokens[4], Token::Symbol(Symbol::Semicolon, _)));
}
