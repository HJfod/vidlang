
use crate::{
    entities::{codebase::Span, names::NameId}, 
    tokens::{token::Symbol, tokenstream::Tokens}
};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum StringComp {
    String(String),
    Expr(Expr),
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum TyExpr {
    Named {
        name: Ident,
        generics: Option<Vec<TyExpr>>,
        span: Span,
    },
    // `A<B>::C<D>`
    Access {
        from: Box<TyExpr>,
        associate: Ident,
        generics: Option<Vec<TyExpr>>,
        span: Span,
    },
}

impl TyExpr {
    pub fn span(&self) -> Span {
        match self {
            Self::Named { span, .. } => *span,
            Self::Access { span, .. } => *span,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Ident(pub NameId, pub Span);

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum LogicChainType {
    And,
    Or,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum FunctionParamKind {
    /// Ordinary function param that is passed by ownership / copy
    Normal,
    /// Function param that is passed by (mutable) reference
    Ref,
    /// Constant-time function param
    Const,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct FunctionParam {
    pub name: Ident,
    pub ty: Option<TyExpr>,
    pub default_value: Option<Box<Expr>>,
    pub kind: FunctionParamKind,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Expr {
    // Literals
    Int(u64, Span),
    Float(f64, Span),
    String(Vec<StringComp>, Span),
    Ident(Ident),
    Tuple(Vec<Expr>, Span),

    // `let a: B = 5`
    Var {
        name: Ident,
        ty: Option<TyExpr>,
        value: Option<Box<Expr>>,
        is_const: bool,
        span: Span,
    },
    Function {
        name: Ident,
        generics: Option<Vec<(Ident, Option<TyExpr>)>>,
        params: Vec<FunctionParam>,
        return_ty: Option<TyExpr>,
        body: Box<Expr>,
        is_clip: bool,
        span: Span,
    },
    ArrowFunction {
        params: Vec<FunctionParam>,
        body: Box<Expr>,
        span: Span,
    },

    // `a(b, c: 5)`
    Call {
        target: Box<Expr>,
        args: Vec<(Option<Ident>, Expr)>,
        op: Option<(Symbol, Span)>,
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
        op: (Symbol, Span),
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
    Return(Option<Box<Expr>>, Span),
    Yield(Box<Expr>, Span),
    // `{ .. }`
    Block(Vec<Expr>, Span),
    Await(Box<Expr>, Span),
}

impl Expr {
    pub fn parse(tokens: &mut Tokens, args: ParseArgs) -> Self {
        Self::parse_binop(tokens, args)
    }
    pub fn requires_semicolon(&self) -> bool {
        match self {
            Self::Int(..) => true,
            Self::Float(..) => true,
            Self::String(..) => true,
            Self::Ident(..) => true,
            Self::Tuple(..) => true,

            Self::Var { .. } => true,
            Self::Function { body, .. } => body.requires_semicolon(),
            Self::ArrowFunction { body, .. } => body.requires_semicolon(),

            Self::Call { .. } => true,
            Self::FieldAccess { .. } => true,
            Self::Assign { .. } => true,
            Self::LogicChain { .. } => true,

            Self::If { truthy, falsy, .. } =>
                falsy.as_ref()
                    .map(|f| f.requires_semicolon())
                    .unwrap_or(truthy.requires_semicolon()),
            Self::Return(value, ..) => value.as_ref()
                .map(|v| v.requires_semicolon())
                .unwrap_or(false),
            Self::Yield(value, ..) => value.requires_semicolon(),
            Self::Block(..) => false,
            Self::Await(value, _) => value.requires_semicolon(),
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Self::Int(_, span) => *span,
            Self::Float(_, span) => *span,
            Self::String(_, span) => *span,
            Self::Ident(ident) => ident.1,
            Self::Tuple(_, span) => *span,
            Self::Var { span, .. } => *span,
            Self::Function { span, .. } => *span,
            Self::ArrowFunction { span, .. } => *span,
            Self::Call { span, .. } => *span,
            Self::FieldAccess { span, .. } => *span,
            Self::Assign { span, .. } => *span,
            Self::LogicChain { span, .. } => *span,
            Self::If { span, .. } => *span,
            Self::Return(_, span) => *span,
            Self::Yield(_, span) => *span,
            Self::Block(_, span) => *span,
            Self::Await(_, span) => *span,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ParseArgs {
    // Useful for tests
    pub allow_non_definitions_at_root: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ParseArgs {
    fn default() -> Self {
        Self {
            allow_non_definitions_at_root: false,
        }
    }
}

#[derive(Debug)]
pub struct Ast(Vec<Expr>);
impl Ast {
    pub fn parse(tokens: &mut Tokens, args: ParseArgs) -> Ast {
        Ast(Expr::parse_semicolon_expr_list(tokens, !args.allow_non_definitions_at_root, args))
    }
    pub fn exprs(&self) -> &[Expr] {
        &self.0
    }
}

#[test]
fn invalid_parses() {
    use crate::entities::codebase::Codebase;
    use crate::entities::messages::Messages;
    use crate::entities::names::Names;

    let test_expr = |data: &str| {
        let mut codebase = Codebase::new();
        let id = codebase.add_memory("invalid_parses", data);

        let names = Names::new();
        let messages = Messages::new();
        codebase.parse_all(names.clone(), messages.clone(), ParseArgs {
            allow_non_definitions_at_root: true,
        });

        assert!(
            messages.counts().0 > 0,
            "`{data}` didn't result in errors:\n{}\ninstead got ast: {:#?}",
            messages.to_test_string(&codebase),
            codebase.fetch(id).ast()
        );
    };

    test_expr("a b");
    test_expr("(");
    test_expr("a +");
    test_expr("function a() -> {}");
    test_expr("clip a() -> A {}");
}

#[test]
fn parse() {
    use crate::entities::codebase::Codebase;
    use crate::entities::names::Names;
    use crate::entities::messages::Messages;

    let mut codebase = Codebase::new();
    let names = Names::new();
    let messages = Messages::new();

    let id = codebase.add_memory("test_parse", r#"
        let x = 8;
        if x > 5 {
            x += hi_guys();
        }
    "#);
    codebase.parse_all(names.clone(), messages.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });
    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );

    let ast_exprs = &codebase.fetch(id).ast().unwrap().0;
    assert_eq!(ast_exprs.len(), 2);

    assert_eq!(*ast_exprs, vec![
        Expr::Var {
            name: Ident(names.add("x"), Span::zero(id)),
            ty: None,
            value: Some(Box::from(Expr::Int(8, Span::zero(id)))),
            span: Span::zero(id),
            is_const: false,
        },
        Expr::If {
            clause: Box::from(Expr::Call {
                target: Box::from(Expr::Ident(Ident(
                    names.builtin_binop_name(Symbol::More),
                    Span::zero(id)
                ))),
                args: vec![
                    (None, Expr::Ident(Ident(names.add("x"), Span::zero(id)))),
                    (None, Expr::Int(5, Span::zero(id))),
                ],
                op: Some((Symbol::More, Span::zero(id))),
                span: Span::zero(id)
            }),
            truthy: Box::from(Expr::Block(vec![
                Expr::Assign {
                    target: Box::from(Expr::Ident(Ident(names.add("x"), Span::zero(id)))),
                    value: Box::from(Expr::Call {
                        target: Box::from(Expr::Ident(Ident(names.add("hi_guys"), Span::zero(id)))),
                        args: vec![],
                        op: None,
                        span: Span::zero(id)
                    }),
                    op: (Symbol::AddAssign, Span::zero(id)),
                    span: Span::zero(id)
                }
            ], Span::zero(id))),
            falsy: None,
            span: Span::zero(id)
        }
    ]);
}
