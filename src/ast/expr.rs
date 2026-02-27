
use crate::{
    pools::{codebase::Span, exprs::{ExprId, Exprs}, names::NameId}, 
    tokens::{token::Symbol, tokenstream::Tokens}
};

#[derive(Debug)]
pub enum StringComp {
    String(String),
    Expr(ExprId),
}

#[derive(Debug)]
pub struct Ident(pub NameId, pub Span);

#[derive(Debug)]
pub enum LogicChainType {
    And,
    Or,
}

#[derive(Debug)]
pub enum FunctionParamKind {
    /// Ordinary function param that is passed by ownership / copy
    Normal,
    /// Function param that is passed by (mutable) reference
    Ref,
    /// Constant-time function param
    Const,
}

#[derive(Debug)]
pub struct FunctionParam {
    pub name: Ident,
    pub ty: Option<ExprId>,
    pub default_value: Option<ExprId>,
    pub kind: FunctionParamKind,
}

#[derive(Debug)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug)]
pub enum Expr {
    Bool(bool, Span),
    Int(u64, Span),
    Float(f64, Span),
    String(Vec<StringComp>, Span),
    Ident(Ident),
    Tuple(Vec<ExprId>, Span),

    // `let a: B = 5`
    Var {
        visibility: Visibility,
        name: Ident,
        ty: Option<ExprId>,
        value: Option<ExprId>,
        is_const: bool,
        span: Span,
    },
    Function {
        visibility: Visibility,
        name: Ident,
        generics: Option<Vec<(Ident, Option<ExprId>)>>,
        params: Vec<FunctionParam>,
        return_ty: Option<ExprId>,
        body: ExprId,
        is_clip: bool,
        span: Span,
    },
    ArrowFunction {
        params: Vec<FunctionParam>,
        body: ExprId,
        span: Span,
    },

    // `a(b, c: 5)`
    Call {
        target: ExprId,
        args: Vec<(Option<Ident>, ExprId)>,
        op: Option<(Symbol, Span)>,
        span: Span,
    },
    // `a.b`
    FieldAccess {
        target: ExprId,
        field: Ident,
        span: Span,
    },
    // `a = 5`
    Assign {
        target: ExprId,
        value: ExprId,
        op: (Symbol, Span),
        span: Span,
    },
    // `a and b and c`
    LogicChain {
        values: Vec<ExprId>,
        ty: LogicChainType,
        span: Span,
    },

    // `if a { b } else { c }`
    If {
        clause: ExprId,
        truthy: ExprId,
        falsy: Option<ExprId>,
        span: Span,
    },
    Return(Option<ExprId>, Span),
    Yield(ExprId, Span),
    // `{ .. }`
    Block(Vec<ExprId>, Span),
    Await(ExprId, Span),

    TyNamed {
        name: Ident,
        generics: Option<Vec<ExprId>>,
        span: Span,
    },
    // `A<B>::C<D>`
    TyAccess {
        from: ExprId,
        associate: Ident,
        generics: Option<Vec<ExprId>>,
        span: Span,
    },
}

impl Expr {
    pub fn parse(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs) -> ExprId {
        Self::parse_binop(tokens, exprs.clone(), args)
    }
    pub fn requires_semicolon(&self, exprs: Exprs) -> bool {
        let sub_requires = |id: ExprId| {
            exprs.exec(id, |e| e.requires_semicolon(exprs.clone()))
        };
        match self {
            Self::Bool(..) => true,
            Self::Int(..) => true,
            Self::Float(..) => true,
            Self::String(..) => true,
            Self::Ident(..) => true,
            Self::Tuple(..) => true,

            Self::Var { .. } => true,
            Self::Function { body, .. } => sub_requires(*body),
            Self::ArrowFunction { body, .. } => sub_requires(*body),

            Self::Call { .. } => true,
            Self::FieldAccess { .. } => true,
            Self::Assign { .. } => true,
            Self::LogicChain { .. } => true,

            Self::If { truthy, falsy, .. } => sub_requires(falsy.unwrap_or(*truthy)),
            Self::Return(value, ..) => value.map(sub_requires).unwrap_or(true),
            Self::Yield(value, ..) => sub_requires(*value),
            Self::Block(..) => false,
            Self::Await(value, _) => sub_requires(*value),

            Self::TyAccess { .. } => true,
            Self::TyNamed { .. } => true,
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Self::Bool(_, span) => *span,
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
            Self::TyNamed { span, .. } => *span,
            Self::TyAccess { span, .. } => *span,
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
pub struct Ast(Vec<ExprId>);
impl Ast {
    pub fn parse(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs) -> Ast {
        Ast(Expr::parse_semicolon_expr_list(tokens, !args.allow_non_definitions_at_root, exprs, args))
    }
    pub fn exprs(&self) -> &[ExprId] {
        &self.0
    }
}

#[test]
fn invalid_parses() {
    use crate::pools::codebase::Codebase;
    use crate::pools::messages::Messages;
    use crate::pools::names::Names;

    let test_expr = |data: &str| {
        let mut codebase = Codebase::new();
        let id = codebase.add_memory("invalid_parses", data);

        let names = Names::new();
        let messages = Messages::new();
        let exprs = Exprs::new();
        codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs {
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
    use crate::pools::codebase::Codebase;
    use crate::pools::names::Names;
    use crate::pools::messages::Messages;

    let mut codebase = Codebase::new();
    let names = Names::new();
    let exprs = Exprs::new();
    let messages = Messages::new();

    let id = codebase.add_memory("test_parse", r#"
        let x = 8;
        if x > 5 {
            x += hi_guys();
        }
    "#);
    codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });
    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );

    let ast_exprs = &codebase.fetch(id).ast().unwrap().0;
    assert_eq!(ast_exprs.len(), 2);

    // let a = ast_exprs.into_iter();
    // let Some(Expr::Var {
    //     visibility: Visibility::Private,
    //     name: Ident(names.add("x"), _),
    //     ty: None,
    //     value: Some(value),
    //     span: _,
    //     is_const: false,
    // }) = a.next();
    // let v = 

    // assert_eq!(*ast_exprs.clone(), vec![
    //     Expr::Var {
    //         visibility: Visibility::Private,
    //         name: Ident(names.add("x"), Span::zero(id)),
    //         ty: None,
    //         value: Some(Box::from(Expr::Int(8, Span::zero(id)))),
    //         span: Span::zero(id),
    //         is_const: false,
    //     },
    //     Expr::If {
    //         clause: Box::from(Expr::Call {
    //             target: Box::from(Expr::Ident(Ident(
    //                 names.builtin_binop_name(Symbol::More),
    //                 Span::zero(id)
    //             ))),
    //             args: vec![
    //                 (None, Expr::Ident(Ident(names.add("x"), Span::zero(id)))),
    //                 (None, Expr::Int(5, Span::zero(id))),
    //             ],
    //             op: Some((Symbol::More, Span::zero(id))),
    //             span: Span::zero(id)
    //         }),
    //         truthy: Box::from(Expr::Block(vec![
    //             Expr::Assign {
    //                 target: Box::from(Expr::Ident(Ident(names.add("x"), Span::zero(id)))),
    //                 value: Box::from(Expr::Call {
    //                     target: Box::from(Expr::Ident(Ident(names.add("hi_guys"), Span::zero(id)))),
    //                     args: vec![],
    //                     op: None,
    //                     span: Span::zero(id)
    //                 }),
    //                 op: (Symbol::AddAssign, Span::zero(id)),
    //                 span: Span::zero(id)
    //             }
    //         ], Span::zero(id))),
    //         falsy: None,
    //         span: Span::zero(id)
    //     }
    // ]);
}
