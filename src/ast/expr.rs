
use crate::{
    entities::{codebase::Span, names::NameId}, 
    tokens::{token::Symbol, tokenstream::Tokens}
};

#[derive(Debug)]
pub enum StringComp {
    String(String),
    Expr(Expr),
}

#[derive(Debug)]
pub enum TyExpr {
    Named(NameId, Span),
}

#[derive(Debug)]
pub struct Ident(pub NameId, pub Span);

#[derive(Debug)]
pub enum LogicChainType {
    And,
    Or,
}

#[derive(Debug)]
pub enum Expr {
    // Literals
    Int(u64, Span),
    Float(f64, Span),
    String(Vec<StringComp>, Span),
    Ident(Ident),

    // `let a: B = 5`
    VarDef {
        name: Ident,
        ty: Option<TyExpr>,
        value: Option<Box<Expr>>,
        span: Span,
    },

    // `a(b, c: 5)`
    Call {
        target: Box<Expr>,
        args: Vec<(Option<Ident>, Expr)>,
        op: Option<Symbol>,
        span: Span,
    },
    // `a.b`
    FieldAccess {
        target: Box<Expr>,
        field: Ident,
        span: Span,
    },
    // `a = 5`
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        op: Option<Symbol>,
        span: Span,
    },
    // `a and b and c`
    LogicChain {
        values: Vec<Expr>,
        ty: LogicChainType,
        span: Span,
    },

    // `if a { b } else { c }`
    If {
        clause: Box<Expr>,
        truthy: Box<Expr>,
        falsy: Option<Box<Expr>>,
        span: Span,
    },
    // `{ .. }`
    Block(Vec<Expr>, Span),
}

impl Expr {
    pub fn parse(tokens: &mut Tokens) -> Self {
        Self::parse_binop(tokens)
    }
    pub fn requires_semicolon(&self) -> bool {
        match self {
            Self::Int(_, _) => true,
            Self::Float(_, _) => true,
            Self::String(_, _) => true,
            Self::Ident(_) => true,

            Self::VarDef { name: _, ty: _, value: _, span: _ } => true,

            Self::Call { target: _, args: _, op: _, span: _ } => true,
            Self::FieldAccess { target: _, field: _, span: _ } => true,
            Self::Assign { target: _, value: _, op: _, span: _ } => true,
            Self::LogicChain { values: _, ty: _, span: _ } => true,

            Self::If { clause: _, truthy, falsy, span: _ } =>
                falsy.as_ref()
                    .map(|f| f.requires_semicolon())
                    .unwrap_or(truthy.requires_semicolon()),
            Self::Block(_, _) => false,
        }
    }
}

#[derive(Debug)]
pub struct Ast(Vec<Expr>);
impl Ast {
    pub fn parse(tokens: &mut Tokens) -> Ast {
        Ast(Expr::parse_semicolon_list(Expr::parse_definition, tokens))
    }
}

#[test]
fn test_parse() {
    use crate::entities::codebase::Codebase;
    use crate::entities::names::Names;
    use crate::entities::messages::Messages;
    use std::assert_matches;

    let mut codebase = Codebase::new();
    let names = Names::new();
    let messages = Messages::new();

    let id = codebase.add_memory("test_parse", r#"
        let x = 0;
        if x > 5 {
            x += 1;
        }
    "#);
    codebase.parse_all(names, messages.clone());
    assert!(messages.count_total() == 0);

    let ast_exprs = &codebase.fetch(id).ast().unwrap().0;
    assert_eq!(ast_exprs.len(), 2);

    assert_matches!(&ast_exprs[0], Expr::VarDef { name: _, ty: None, value: Some(_), span: _ });
}
