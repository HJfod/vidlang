
use crate::{pools::{
    names::NameId,
    modules::Span
}, tokens::tokenstream::Tokens};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BracketType {
    Parentheses,
    Brackets,
    Braces,
}

impl BracketType {
    pub fn from_open(ch: char) -> Option<BracketType> {
        match ch {
            '(' => Some(BracketType::Parentheses),
            '[' => Some(BracketType::Brackets),
            '{' => Some(BracketType::Braces),
            _   => None,
        }
    }
    pub fn open(&self) -> char {
        match self {
            BracketType::Parentheses   => '(',
            BracketType::Brackets      => '[',
            BracketType::Braces        => '{',
        }
    }
    pub fn close(&self) -> char {
        match self {
            BracketType::Parentheses   => ')',
            BracketType::Brackets      => ']',
            BracketType::Braces        => '}',
        }
    }
    pub fn expected_name(&self) -> &'static str {
        match self {
            BracketType::Parentheses => "parenthesized expression",
            BracketType::Brackets => "bracketed expression",
            BracketType::Braces => "braced expression",
        }
    }
}

#[derive(Debug, Clone, Copy, strum_macros::Display, strum_macros::EnumString, PartialEq)]
#[strum(serialize_all="snake_case")]
pub enum Symbol {
    // Keywords
    Let, Const, Type, Function, Effect, Clip, Module,
    Trait, Impl, Struct, Unit, Enum,
    Using, Private, Public, Ref,
    InvokeIntrinsic,
    #[strum(to_string="typeof")]
    TypeOf,
    Match, If, Then, Else, While, For, In, Loop, Await, Return, Yield,
    And, Or,
    True, False, None,
    Macro, Codegen,
    // Operators
    #[strum(to_string="=")]
    Assign,
    #[strum(to_string=":=")]
    WalrusAssign,
    #[strum(to_string="+=")]
    AddAssign,
    #[strum(to_string="-=")]
    SubAssign,
    #[strum(to_string="**")]
    Power,
    #[strum(to_string="*")]
    Mul,
    #[strum(to_string="/")]
    Div,
    #[strum(to_string="mod")]
    Mod,
    #[strum(to_string="+")]
    Plus,
    #[strum(to_string="-")]
    Minus,
    #[strum(to_string="==")]
    Eq,
    #[strum(to_string="!=")]
    Neq,
    #[strum(to_string="<")]
    Less,
    #[strum(to_string="<=")]
    Leq,
    #[strum(to_string=">")]
    More,
    #[strum(to_string=">=")]
    Meq,
    #[strum(to_string="->")]
    Arrow,
    #[strum(to_string="=>")]
    FatArrow,
    #[strum(to_string=":")]
    Colon,
    #[strum(to_string="::")]
    Scope,
    #[strum(to_string=".")]
    Dot,
    #[strum(to_string="...")]
    DotDotDot,
    #[strum(to_string=",")]
    Comma,
    #[strum(to_string=";")]
    Semicolon,
    #[strum(to_string="!")]
    Exclamation,
    #[strum(to_string="?")]
    Question,
}

impl Symbol {
    pub fn is_reserved(self) -> bool {
        matches!(self,
            // maybe add support for custom units (px, %, etc.) in the future
            Symbol::Unit |
            // might want to have an explicit struct keyword
            Symbol::Struct |
            // add support for macros in the future
            Symbol::Macro | Symbol::Codegen | 
            // mayber add support for traits in the future (if I decide to add generics aswell)
            Symbol::Trait | Symbol::Impl
        )
    }
}

#[derive(Debug)]
pub enum StrLitComp {
    String(String),
    Component(Tokens),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Duration {
    Seconds(f64),
    Frames(u64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatLitType {
    Number,
    Percentage,
}

#[derive(Debug)]
pub enum Token {
    // Integer literals don't take into account the '-' ever, so we can parse 
    // into an u64 instead for more precision
    Int(u64, Span),
    // todo: might actually want to make percentages a distinct type from floats
    Float(f64, FloatLitType, Span),
    Duration(Duration, Span),
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
            Self::Float(_, _, span) => *span,
            Self::Duration(_, span) => *span,
            Self::String(_, span) => *span,
            Self::Ident(_, span) => *span,
            Self::Symbol(_, span) => *span,
            Self::Bracketed(_, _, span) => *span,
            Self::Attribute(_, _, span) => *span,
        }
    }
    pub fn expected_name(&self) -> &'static str {
        match self {
            Self::Int(..)           => "integer",
            Self::Float(..)         => "float",
            Self::Duration(..)      => "duration",
            Self::String(..)        => "string",
            Self::Ident(..)         => "identifier",
            Self::Symbol(..)        => "keyword or operator",
            Self::Bracketed(ty, ..) => ty.expected_name(),
            Self::Attribute(..)     => "attribute",
        }
    }
}

#[test]
fn symbols() {
    use std::str::FromStr;

    assert_eq!(Symbol::from_str("+="), Ok(Symbol::AddAssign));
    assert_eq!(Symbol::from_str("let"), Ok(Symbol::Let));
    assert_eq!(Symbol::from_str("->"), Ok(Symbol::Arrow));
    assert_eq!(Symbol::from_str("=>"), Ok(Symbol::FatArrow));
    assert!(Symbol::from_str("++=").is_err());
    assert!(Symbol::from_str("=<").is_err());
    assert!(Symbol::from_str("-->").is_err());
}
