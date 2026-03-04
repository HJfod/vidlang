
use std::str::FromStr;

use crate::{pools::{
    messages::{Message, Messages}, modules::{Span, SrcIterator}, names::Names
}, tokens::{token::{BracketType, Duration, StrLitComp, Symbol, Token}, tokenstream::Tokens}};
use unicode_xid::UnicodeXID;

impl Token {
    fn skip_to_next(iter: &mut SrcIterator) {
        while let Some(c) = iter.peek() {
            if c.is_whitespace() {
                iter.next();
            }
            // Comments (no doc comments)
            else if c == '/' && iter.peek_n(1) == Some('/') && iter.peek_n(2) != Some('/') {
                // Skip everything until end of line
                for n in &mut *iter {
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
    fn peek_and_next_ident(iter: &mut SrcIterator) -> Option<(String, Span)> {
        if iter.peek().is_some_and(|c| c.is_xid_start()) {
            let start = iter.index();
            let c = iter.next().unwrap();
            let rest = iter.next_while(UnicodeXID::is_xid_continue);
            Some((String::from(c) + &rest, iter.span_from(start)))
        }
        else {
            None
        }
    }
    pub fn parse(iter: &mut SrcIterator, names: &mut Names, messages: &mut Messages) -> Option<Token> {
        Self::skip_to_next(iter);
        let start = iter.index();
        let c = iter.next()?;

        // todo: Doc comments
        // if c == '/' && iter.peek_n(1) == Some('/') && iter.peek_n(2) == Some('/') {
        //     // Consume the other ones
        //     iter.next();
        //     iter.next();

        // }

        // String literals
        if c == '"' {
            let mut comps = Vec::new();
            'parse_comps: loop {
                let Some(c) = iter.next() else {
                    messages.add(Message::expected("closing quote", "eof", iter.head()));
                    break 'parse_comps;
                };
                // Closing quote
                if c == '"' {
                    break 'parse_comps;
                }
                // Interpolated expressions like `"2 + 2 = {2 + 2}"`
                if c == '{' {
                    let mut tokens = Vec::new();

                    Self::skip_to_next(iter);
                    let tokens_start = iter.head();
                    'parse_expr: loop {
                        Self::skip_to_next(iter);
                        if iter.peek() == Some('}') {
                            iter.next();
                            break 'parse_expr;
                        }
                        // No need to check for EOF since this checks it for us
                        let Some(tk) = Self::parse(iter, names, messages) else {
                            messages.add(Message::expected(
                                "closing brace", "eof", iter.head()
                            ));
                            break 'parse_comps;
                        };
                        tokens.push(tk);
                    }
                    comps.push(StrLitComp::Component(Tokens::new(tokens, "closing brace", tokens_start)))
                }
                // Otherwise push the char (or escaped char)
                else {
                    let char_start = iter.index();
                    let char_to_push = match c {
                        '\\' => {
                            let Some(escaped) = iter.next() else {
                                messages.add(Message::expected(
                                    "escaped character", "eof", iter.head()
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
                                    messages.add(Message::new_error(
                                        format!("unknown escape sequence \"\\{c}\""),
                                        iter.span_from(char_start)
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
            return Some(Token::String(comps, iter.span_from(start)))
        }

        // Number literals
        if c.is_ascii_digit() {
            let mut num_str = String::from(c) + &iter.next_while(|c| c.is_ascii_digit());
            // Floating point number
            if iter.peek() == Some('.') {
                num_str.push(iter.next().unwrap());
                num_str.push_str(&iter.next_while(|c| c.is_ascii_digit()));
            }
            let num_span = iter.span_from(start);
            let maybe_unit = Self::peek_and_next_ident(iter);

            // Some helpers
            let parse_as_f64 = |messages: &mut Messages| match num_str.parse::<f64>() {
                Ok(v) => v,
                Err(e) => {
                    messages.add(Message::new_error(format!("invalid number: {e}"), num_span));
                    0.0
                }
            };
            let parse_as_u64 = |messages: &mut Messages| match num_str.parse::<u64>() {
                Ok(v) => v,
                Err(e) => {
                    messages.add(Message::new_error(format!("invalid integer: {e}"), num_span));
                    0
                }
            };

            // Units
            if let Some((unit, unit_span)) = maybe_unit {
                let require_as_u64 = |messages: &mut Messages| if num_str.contains('.') {
                    messages.add(Message::new_error(
                        format!("unit {unit} may only be specified on integers"),
                        unit_span
                    ));
                    0
                }
                else {
                    parse_as_u64(messages)
                };
                return Some(Token::Duration(match unit.as_str() {
                    "ms"             => Duration::Seconds(parse_as_f64(messages) / 1000.0),
                    "s"              => Duration::Seconds(parse_as_f64(messages)),
                    "frm" | "frames" => Duration::Frames(require_as_u64(messages)),
                    _ => {
                        messages.add(Message::new_error(format!("unknown unit {unit}"), unit_span));
                        Duration::Frames(0)
                    }
                }, iter.span_from(start)));
            }
            // Otherwise parse float if the token was one
            if num_str.contains('.') {
                return Some(Token::Float(parse_as_f64(messages), num_span))
            }
            // Otherwise this is an integer
            return Some(Token::Int(parse_as_u64(messages), num_span));
        }

        // Identifiers & keywords
        // Note that '_' is not XID_Start
        if c.is_xid_start() || c == '_' {
            let ident = String::from(c) + &iter.next_while(UnicodeXID::is_xid_continue);
            let span = iter.span_from(start);
            return Some(match Symbol::from_str(&ident) {
                Ok(sym) => {
                    // Return reserved keywords as identifiers
                    if sym.is_reserved() {
                        messages.add(Message::new_error(
                            format!("keyword '{sym}' is reserved"),
                            span
                        ));
                        Token::Ident(names.add(&ident), span)
                    }
                    else {
                        Token::Symbol(sym, span)
                    }
                }
                Err(_) => Token::Ident(names.add(&ident), span),
            });
        }

        // Brackets
        if let Some(ty) = BracketType::from_open(c) {
            let mut contents = Vec::new();

            Self::skip_to_next(iter);
            let contents_start = iter.head();
            loop {
                Self::skip_to_next(iter);
                if iter.peek() == Some(ty.close()) {
                    iter.next();
                    break;
                }
                let Some(tk) = Self::parse(iter, names, messages) else {
                    messages.add(Message::expected(
                        format!("'{}'", ty.close()), "eof", iter.head()
                    ));
                    break;
                };
                contents.push(tk);
            }
            return Some(Token::Bracketed(
                ty,
                Box::from(Tokens::new(contents, format!("'{}'", ty.close()), contents_start)),
                iter.span_from(start)
            ));
        }

        // Attributes
        if c == '@' {
            let ident = match Self::peek_and_next_ident(iter) {
                Some(i) => names.add(&i.0),
                None => {
                    messages.add(Message::expected_what(
                        "identifier for attribute", iter.head()
                    ));
                    names.missing()
                }
            };
            // Args for attributes
            let args = iter.peek().is_some_and(|c| c == '(').then(|| {
                let Some(Token::Bracketed(_, contents, _)) = Self::parse(iter, names, messages) else {
                    unreachable!("Token that starts with '(' was not Token::Bracketed");
                };
                contents
            });
            return Some(Token::Attribute(ident, args, iter.span_from(start)));
        }

        // Otherwise it must be an operator or an invalid character
        let mut sym = String::from(c);

        let peek = iter.peek();

        #[allow(clippy::if_same_then_else)]
        // Everything like +=, -=, etc.
        if "=:+-*/%^~|<>?!".contains(c) && peek == Some('=') {
            sym.push(iter.next().unwrap());
        }
        // Arrows (=>, ->)
        else if (c == '=' || c == '-') && peek == Some('>') {
            sym.push(iter.next().unwrap());
        }
        // **
        else if c == '*' && peek == Some('*') {
            sym.push(iter.next().unwrap());
        }
        // ??
        else if c == '?' && peek == Some('?') {
            sym.push(iter.next().unwrap());
        }
        // ::
        else if c == ':' && peek == Some(':') {
            sym.push(iter.next().unwrap());
        }
        // All of the dots
        else if c == '.' {
            while iter.peek() == Some('.') {
                sym.push(iter.next().unwrap());
            }
        }

        match Symbol::from_str(&sym) {
            Ok(sym) => Some(Token::Symbol(sym, iter.span_from(start))),
            // Simply skip invalid characters
            Err(_) => {
                messages.add(Message::new_error(
                    format!("invalid symbol '{}'", sym),
                    iter.span_from(start)
                ));
                Self::parse(iter, names, messages)
            }
        }
    }
}

#[test]
fn strings() {
    use crate::codebase::Codebase;
    use std::assert_matches;

    let (mut codebase, id) = Codebase::new_with_test_package("strings", r#"
        "String with\nmore lines and\n\tescape sequences!"
        "String with {  interpolation  } and {stuff}{}"
    "#);
    let mut tokens = codebase.tokenize_mod(id).unwrap();

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
            matches!(tks.peek(), Some(Token::Ident(name, _)) if *name == codebase.names.add("interpolation"))
    );
    assert_matches!(interp.next(), Some(StrLitComp::String(s)) if s == " and ");
    assert_matches!(interp.next(),
        Some(StrLitComp::Component(tks)) if tks.peek_n(1).is_none() &&
            matches!(tks.peek(), Some(Token::Ident(name, _)) if *name == codebase.names.add("stuff"))
    );
    assert_matches!(interp.next(), Some(StrLitComp::Component(tks)) if tks.peek().is_none());
}

#[test]
fn tokenizer() {
    use crate::codebase::Codebase;
    let (mut codebase, id) = Codebase::new_with_test_package("test_tokenizer", r#"
        // This is a comment and it should not show up!
        let x += 5;
    "#);
    let tokens = codebase.tokenize_mod(id).unwrap().collect::<Vec<_>>();
    assert!(codebase.messages.count_total() == 0, "{:?}", codebase.messages);
    assert_eq!(tokens.len(), 5);
    assert!(matches!(tokens[0], Token::Symbol(Symbol::Let, _)));
    assert!(matches!(tokens[1], Token::Ident(_, _)));
    assert!(matches!(tokens[2], Token::Symbol(Symbol::AddAssign, _)));
    assert!(matches!(tokens[3], Token::Int(_, _)));
    assert!(matches!(tokens[4], Token::Symbol(Symbol::Semicolon, _)));
}

#[test]
fn units() {
    use crate::codebase::Codebase;
    let (mut codebase, id) = Codebase::new_with_test_package(
        "units",
        r#"
            5s 60.6ms 17frames
            10.2frames 20unknown
        "#
    );
    let mut tokens = codebase.tokenize_mod(id).unwrap();
    assert!(matches!(tokens.next(), Some(Token::Duration(Duration::Seconds(5.0), _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(Duration::Seconds(0.0606), _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(Duration::Frames(17), _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(_, _))));
    assert!(matches!(tokens.next(), Some(Token::Duration(_, _))));
    assert!(codebase.messages.count_total() == 2, "{:?}", codebase.messages);
}
