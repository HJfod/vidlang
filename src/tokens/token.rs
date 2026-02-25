
use crate::{entities::{
    names::NameId,
    src::Span
}, tokens::tokenstream::Tokens};

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

#[derive(Debug, Clone, Copy, strum_macros::Display, strum_macros::EnumString, PartialEq, Eq)]
#[strum(serialize_all="snake_case")]
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
    #[strum(to_string=":")]
    Colon,
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
    Attribute(NameId, Option<Box<Tokens>>, Span),
}

impl Token {
    pub fn span(&self) -> Span {
        match self {
            Self::Int(_, span) => *span,
            Self::Float(_, span) => *span,
            Self::String(_, span) => *span,
            Self::Ident(_, span) => *span,
            Self::Symbol(_, span) => *span,
            Self::Bracketed(_, _, span) => *span,
            Self::Attribute(_, _, span) => *span,
        }
    }
    pub fn expected_name(&self) -> &'static str {
        match self {
            Self::Int(_, _)          => "integer literal",
            Self::Float(_, _)        => "floating point literal",
            Self::String(_, _)       => "string literal",
            Self::Ident(_, _)        => "identifier",
            Self::Symbol(_, _)       => "keyword or operator",
            Self::Bracketed(_, _, _) => "bracketed sequence",
            Self::Attribute(_, _, _) => "attribute",
        }
    }
}

#[test]
fn test_symbols() {
    use std::str::FromStr;

    assert_eq!(Symbol::from_str("+="), Ok(Symbol::SumAssign));
    assert_eq!(Symbol::from_str("let"), Ok(Symbol::Let));
    assert!(Symbol::from_str("++=").is_err());
}
