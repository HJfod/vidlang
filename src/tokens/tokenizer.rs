
use std::str::FromStr;

use crate::{pools::{
    codebase::{Span, SrcIterator}, messages::{Message, Messages}, names::Names
}, tokens::{token::{BracketType, Duration, StrLitComp, Symbol, Token}, tokenstream::Tokens}};
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
    fn peek_and_next_ident(&mut self) -> Option<(String, Span)> {
        if self.iter.peek().is_some_and(|c| c.is_xid_start()) {
            let start = self.iter.index();
            let c = self.iter.next().unwrap();
            let rest = self.iter.next_while(UnicodeXID::is_xid_continue);
            Some((String::from(c) + &rest, self.iter.span_from(start)))
        }
        else {
            None
        }
    }
}

impl<'s> Iterator for Tokenizer<'s> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.skip_to_next();
        let start = self.iter.index();
        let c = self.iter.next()?;

        // todo: Doc comments
        // if c == '/' && self.iter.peek_n(1) == Some('/') && self.iter.peek_n(2) == Some('/') {
        //     // Consume the other ones
        //     self.iter.next();
        //     self.iter.next();

        // }

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
                    comps.push(StrLitComp::Component(Tokens::new(
                        tokens, "closing brace", tokens_start, self.names.clone(), self.messages.clone()
                    )))
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
                    if comps.last().is_none_or(|c| !matches!(c, StrLitComp::String(_))) {
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
            }
            let num_span = self.iter.span_from(start);
            let maybe_unit = self.peek_and_next_ident();

            // Some helpers
            let parse_as_f64 = || match num_str.parse::<f64>() {
                Ok(v) => v,
                Err(e) => {
                    self.messages.add(Message::new_error(format!("invalid number: {e}"), num_span));
                    0.0
                }
            };
            let parse_as_u64 = || match num_str.parse::<u64>() {
                Ok(v) => v,
                Err(e) => {
                    self.messages.add(Message::new_error(format!("invalid integer: {e}"), num_span));
                    0
                }
            };

            // Units
            if let Some((unit, unit_span)) = maybe_unit {
                let require_as_u64 = || if num_str.contains('.') {
                    self.messages.add(Message::new_error(
                        format!("unit {unit} may only be specified on integers"),
                        unit_span
                    ));
                    0
                }
                else {
                    parse_as_u64()
                };
                return Some(Token::Duration(match unit.as_str() {
                    "ms"             => Duration::Seconds(parse_as_f64() / 1000.0),
                    "s"              => Duration::Seconds(parse_as_f64()),
                    "frm" | "frames" => Duration::Frames(require_as_u64()),
                    _ => {
                        self.messages.add(Message::new_error(format!("unknown unit {unit}"), unit_span));
                        Duration::Frames(0)
                    }
                }, self.iter.span_from(start)));
            }
            // Otherwise parse float if the token was one
            if num_str.contains('.') {
                return Some(Token::Float(parse_as_f64(), num_span))
            }
            // Otherwise this is an integer
            return Some(Token::Int(parse_as_u64(), num_span));
        }

        // Identifiers & keywords
        // Note that '_' is not XID_Start
        if c.is_xid_start() || c == '_' {
            let ident = String::from(c) + &self.iter.next_while(UnicodeXID::is_xid_continue);
            let span = self.iter.span_from(start);
            return Some(match Symbol::from_str(&ident) {
                Ok(sym) => {
                    // Return reserved keywords as identifiers
                    if sym.is_reserved() {
                        self.messages.add(Message::new_error(
                            format!("keyword '{sym}' is reserved"),
                            span
                        ));
                        Token::Ident(self.names.add(&ident), span)
                    }
                    else {
                        Token::Symbol(sym, span)
                    }
                }
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
                Box::from(Tokens::new(
                    contents, format!("'{}'", ty.close()), contents_start, self.names.clone(), self.messages.clone()
                )),
                self.iter.span_from(start)
            ));
        }

        // Attributes
        if c == '@' {
            let ident = match self.peek_and_next_ident() {
                Some(i) => self.names.add(&i.0),
                None => {
                    self.messages.add(Message::expected_what(
                        "identifier for attribute", self.iter.head()
                    ));
                    self.names.missing()
                }
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

        let peek = self.iter.peek();

        #[allow(clippy::if_same_then_else)]
        // Everything like +=, -=, etc.
        if "=:+-*/%^~|<>?!".contains(c) && peek == Some('=') {
            sym.push(self.iter.next().unwrap());
        }
        // Arrows (=>, ->)
        else if (c == '=' || c == '-') && peek == Some('>') {
            sym.push(self.iter.next().unwrap());
        }
        // **
        else if c == '*' && peek == Some('*') {
            sym.push(self.iter.next().unwrap());
        }
        // ??
        else if c == '?' && peek == Some('?') {
            sym.push(self.iter.next().unwrap());
        }
        // ::
        else if c == ':' && peek == Some(':') {
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
fn strings() {
    use crate::pools::codebase::Codebase;
    use std::assert_matches;

    let names = Names::new();
    let messages = Messages::new();
    let (codebase, id) = Codebase::new_with_test_package("strings", r#"
        "String with\nmore lines and\n\tescape sequences!"
        "String with {  interpolation  } and {stuff}{}"
    "#);
    let mut tokens = codebase.tokenize(id, names.clone(), messages.clone()).unwrap();

    let Some(Token::String(escaped_vec, _)) = tokens.next() else { panic!() };
    assert_eq!(escaped_vec.len(), 1, "string literal parts: {escaped_vec:?}");
    let Some(StrLitComp::String(esc)) = escaped_vec.into_iter().next() else { panic!() };
    assert_eq!(esc, "String with\nmore lines and\n\tescape sequences!");

    let Some(Token::String(interp_vec, _)) = tokens.next() else { panic!() };
    assert_eq!(interp_vec.len(), 5, "string literal parts: {interp_vec:?}");

    let mut interp = interp_vec.into_iter();
    assert_matches!(interp.next(), Some(StrLitComp::String(s)) if s == "String with ");
    assert_matches!(interp.next(),
        Some(StrLitComp::Component(tks)) if tks.peek_n(1).is_none() &&
            matches!(tks.peek(), Some(Token::Ident(name, _)) if *name == names.add("interpolation"))
    );
    assert_matches!(interp.next(), Some(StrLitComp::String(s)) if s == " and ");
    assert_matches!(interp.next(),
        Some(StrLitComp::Component(tks)) if tks.peek_n(1).is_none() &&
            matches!(tks.peek(), Some(Token::Ident(name, _)) if *name == names.add("stuff"))
    );
    assert_matches!(interp.next(), Some(StrLitComp::Component(tks)) if tks.peek().is_none());
}

#[test]
fn tokenizer() {
    use crate::pools::codebase::Codebase;
    let names = Names::new();
    let messages = Messages::new();
    let (codebase, id) = Codebase::new_with_test_package("test_tokenizer", r#"
        // This is a comment and it should not show up!
        let x += 5;
    "#);
    let tokens = codebase.tokenize(id, names, messages.clone()).unwrap().collect::<Vec<_>>();
    assert!(messages.count_total() == 0, "{messages:?}");
    assert_eq!(tokens.len(), 5);
    assert!(matches!(tokens[0], Token::Symbol(Symbol::Let, _)));
    assert!(matches!(tokens[1], Token::Ident(_, _)));
    assert!(matches!(tokens[2], Token::Symbol(Symbol::AddAssign, _)));
    assert!(matches!(tokens[3], Token::Int(_, _)));
    assert!(matches!(tokens[4], Token::Symbol(Symbol::Semicolon, _)));
}

#[test]
fn units() {
    use crate::pools::codebase::Codebase;
    let names = Names::new();
    let messages = Messages::new();

    let (codebase, id) = Codebase::new_with_test_package(
        "units",
        r#"
            5s 60.6ms 17frames
            10.2frames 20unknown
        "#
    );
    let mut tokens = codebase.tokenize(id, names, messages.clone()).unwrap();
    assert!(matches!(tokens.next(), Some(Token::Duration(Duration::Seconds(5.0), _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(Duration::Seconds(0.0606), _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(Duration::Frames(17), _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(_, _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(_, _))));
    assert!(messages.count_total() == 2, "{messages:?}");
}
