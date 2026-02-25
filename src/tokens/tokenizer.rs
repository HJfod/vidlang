
use std::str::FromStr;

use crate::{tokens::token::{BracketType, StrLitComp, Symbol, Token}, entities::{
    messages::{Message, Messages},
    names::{MISSING_NAME, Names},
    src::SrcIterator
}, tokens::tokenstream::Tokens};
use unicode_xid::UnicodeXID;

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

                    self.skip_to_next();
                    let tokens_start = self.iter.head();
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
                    comps.push(StrLitComp::Component(Tokens::new(tokens, "closing brace", tokens_start)))
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

            self.skip_to_next();
            let contents_start = self.iter.head();
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
                ty,
                Box::from(Tokens::new(contents, format!("'{}'", ty.close()), contents_start)),
                self.iter.span_from(start)
            ));
        }

        // Attributes
        if c == '@' {
            let ident = if self.iter.peek().is_some_and(|c| c.is_xid_start()) {
                self.iter.next();
                self.names.add(&(
                    String::from(c) + &self.iter.next_while(UnicodeXID::is_xid_continue)
                ))
            }
            else {
                self.messages.add(Message::expected_what(
                    "identifier for attribute", self.iter.head()
                ));
                MISSING_NAME
            };
            // Args for attributes
            let args = self.iter.peek().is_some_and(|c| c == '(').then(|| {
                let Some(Token::Bracketed(_, contents, _)) = self.next() else {
                    unreachable!("Token that starts with '(' was not Token::Bracketed");
                };
                contents
            });
            return Some(Token::Attribute(ident, args, self.iter.span_from(start)));
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
