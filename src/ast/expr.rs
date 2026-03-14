
use crate::{
    ast::intrinsics::Intrinsic, codebase::Codebase, pools::{exprs::ExprId, modules::Span, names::NameId}, tokens::{token::{Duration, FloatLitType, Symbol}, tokenstream::Tokens}
};

#[derive(Debug)]
pub enum StringComp {
    String(String),
    Expr(ExprId),
}

#[derive(Debug)]
pub struct Ident(pub NameId, pub Span);

#[derive(Debug)]
pub struct IdentPath(pub Vec<Ident>, pub Span);

impl IdentPath {
    pub fn from_components(comps: &[&str], span: Span, codebase: &mut Codebase) -> IdentPath {
        IdentPath(
            comps.iter().map(|s| Ident(codebase.names.add(s), span)).collect(),
            span
        )
    }
}

#[derive(Debug)]
pub enum UsingIdentItem {
    /// This path segment ends in all items, like `std::ops::{...}`
    AllItems,
    /// This path segment ends in a list of specific items, like `std::ops::{add, sub}`
    Items(Vec<Ident>),
}

#[derive(Debug)]
pub struct UsingIdentPath { 
    pub parent: IdentPath,
    pub item: UsingIdentItem,
    pub span: Span,
}

#[derive(Debug)]
pub enum LogicChainType {
    And,
    Or,
}

#[derive(Debug)]
pub enum FunctionParamKind {
    /// Ordinary function param that is passed by ownership / copy
    Normal,
    /// Constant property (only allowed in functions whose parameters are 
    /// properties, aka clips & effects)
    Const,
}

#[derive(Debug)]
pub struct FunctionParam {
    pub name: Ident,
    pub ty: Option<ExprId>,
    pub default_value: Option<ExprId>,
    pub kind: FunctionParamKind,
    pub from: Vec<Ident>,
}

#[derive(Debug)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug)]
pub enum FunctionType {
    Function,
    Effect,
    Clip,
}

#[derive(Debug)]
pub enum TupleTypeField {
    /// Field with a type and an optional default value
    Field {
        is_const: bool,
        name: Ident,
        ty: ExprId,
        default: Option<ExprId>
    },
    /// Only one of these fields may be active at a time. If type is omitted, 
    /// then the field receives an unique type. May not have a default value
    Enum(Vec<(Ident, Option<ExprId>)>),
}

#[derive(Debug)]
pub enum Expr {
    /// The value `none`
    None(Span),
    Bool(bool, Span),
    Int(u64, Span),
    Float(f64, FloatLitType, Span),
    Duration(Duration, Span),
    String(Vec<StringComp>, Span),
    Ident(IdentPath),
    /// Default value expression, i.e. just the keyword `default`
    DefaultValue(Span),

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
        ty: FunctionType,
        name: IdentPath,
        params: Vec<FunctionParam>,
        return_ty: Option<ExprId>,
        body: ExprId,
        is_const: bool,
        span: Span,
    },
    ArrowFunction {
        params: Vec<FunctionParam>,
        body: ExprId,
        span: Span,
    },
    Module {
        visibility: Visibility,
        name: IdentPath,
        items: Vec<ExprId>,
        span: Span,
    },
    Using {
        visibility: Visibility,
        path: UsingIdentPath,
        span: Span,
    },
    TypeDef {
        visibility: Visibility,
        name: IdentPath,
        ty: ExprId,
        span: Span,
    },

    // Tuple literal `(a: 2, b: 5)` or call `a(b, c: 5)`
    CallOrTuple {
        target: Option<ExprId>,
        /// If no field names are provided, then will implicitly give fields 
        /// numbered names starting from `0`
        args: Vec<(Ident, ExprId)>,
        op: Option<(Symbol, Span)>,
        span: Span,
    },
    InvokeIntrinsic {
        intrinsic: Intrinsic,
        args: Vec<ExprId>,
        span: Span,
    },
    // `a.b` or `a?.b`
    FieldAccess {
        target: ExprId,
        field: IdentPath,
        optional: bool,
        span: Span,
    },
    // `a[b]`
    IndexAccess {
        target: ExprId,
        index: ExprId,
        span: Span,
    },
    // `a = 5`
    Assign {
        target: ExprId,
        value: ExprId,
        op: (Symbol, Span),
        span: Span,
    },
    // `a = from prop1, prop2 { .. }`
    AssignFrom {
        target: ExprId,
        properties: Vec<Ident>,
        body: ExprId,
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

    /// Simple named type like `std::types::Void`
    TyNamed(IdentPath),
    /// Subtype of another type, like `(A -> B)::Return`
    TyAccess {
        from: ExprId,
        item: IdentPath,
        span: Span,
    },
    /// A function with multiple args simply takes a tuple type as its first arg
    TyFunction {
        param: ExprId,
        return_ty: ExprId,
        span: Span,
    },
    TyArray {
        inner: ExprId,
        span: Span,
    },
    TyTuple {
        fields: Vec<TupleTypeField>,
        span: Span,
    },
    TyOptional {
        inner: ExprId,
        span: Span,
    },
    TyRef {
        inner: ExprId,
        span: Span,
    },
    /// Join two types together via `A + B` (results in a new tuple `(A, B)`)
    TyJoin {
        lhs: ExprId,
        rhs: ExprId,
        span: Span,
    },
    TypeOf {
        eval: ExprId,
        span: Span,
    },
}

impl Expr {
    pub fn parse(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        Self::parse_binop(tokens, codebase, args)
    }
    pub fn requires_semicolon(&self, codebase: &Codebase) -> bool {
        let sub_requires = |id: ExprId| {
            codebase.exprs.get(id).requires_semicolon(codebase)
        };
        match self {
            Self::None(..) => true,
            Self::Bool(..) => true,
            Self::Int(..) => true,
            Self::Float(..) => true,
            Self::Duration(..) => true,
            Self::String(..) => true,
            Self::Ident(..) => true,
            Self::DefaultValue(..) => true,

            Self::Var { .. } => true,
            Self::Function { body, .. } => sub_requires(*body),
            Self::ArrowFunction { body, .. } => sub_requires(*body),
            Self::Module { .. } => false,
            Self::Using { .. } => true,
            Self::TypeDef { ty, .. } => sub_requires(*ty),

            Self::CallOrTuple { .. } => true,
            Self::InvokeIntrinsic { .. } => true,
            Self::FieldAccess { .. } => true,
            Self::IndexAccess { .. } => true,
            Self::Assign { .. } => true,
            Self::AssignFrom { .. } => false,
            Self::LogicChain { .. } => true,

            Self::If { truthy, falsy, .. } => sub_requires(falsy.unwrap_or(*truthy)),
            Self::Return(value, ..) => value.map(sub_requires).unwrap_or(true),
            Self::Yield(value, ..) => sub_requires(*value),
            Self::Block(..) => false,
            Self::Await(value, _) => sub_requires(*value),

            Self::TyNamed(..) => true,
            Self::TyAccess { .. } => true,
            Self::TyFunction { .. } => true,
            Self::TyArray { .. } => true,
            Self::TyTuple { .. } => true,
            Self::TyOptional { .. } => true,
            Self::TyRef { .. } => true,
            Self::TyJoin { .. } => true,
            Self::TypeOf { .. } => true,
        }
    }
    pub fn is_stmt_like(&self) -> bool {
        match self {
            Self::None(..) => false,
            Self::Bool(..) => false,
            Self::Int(..) => false,
            Self::Float(..) => false,
            Self::Duration(..) => false,
            Self::String(..) => false,
            Self::Ident(..) => false,
            Self::DefaultValue(..) => false,

            Self::Var { .. } => true,
            Self::Function { .. } => true,
            Self::ArrowFunction { .. } => true,
            Self::Module { .. } => true,
            Self::Using { .. } => true,
            Self::TypeDef { .. } => true,

            Self::CallOrTuple { .. } => false,
            Self::InvokeIntrinsic { .. } => false,
            Self::FieldAccess { .. } => false,
            Self::IndexAccess { .. } => false,
            Self::Assign { .. } => true,
            Self::AssignFrom { .. } => true,
            Self::LogicChain { .. } => false,

            Self::If { .. } => false,
            Self::Return(..) => false,
            Self::Yield(..) => false,
            Self::Block(..) => false,
            Self::Await(..) => false,

            Self::TyNamed(..) => false,
            Self::TyAccess { .. } => false,
            Self::TyFunction { .. } => false,
            Self::TyArray { .. } => false,
            Self::TyTuple { .. } => false,
            Self::TyOptional { .. } => false,
            Self::TyRef { .. } => false,
            Self::TyJoin { .. } => false,
            Self::TypeOf { .. } => false,
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Self::None(span) => *span,
            Self::Bool(_, span) => *span,
            Self::Int(_, span) => *span,
            Self::Float(_, _, span) => *span,
            Self::Duration(_, span) => *span,
            Self::String(_, span) => *span,
            Self::Ident(ident) => ident.1,
            Self::DefaultValue(span) => *span,
            Self::Var { span, .. } => *span,
            Self::Function { span, .. } => *span,
            Self::ArrowFunction { span, .. } => *span,
            Self::Module { span, .. } => *span,
            Self::Using { span, .. } => *span,
            Self::TypeDef { span, .. } => *span,
            Self::CallOrTuple { span, .. } => *span,
            Self::InvokeIntrinsic { span, .. } => *span,
            Self::FieldAccess { span, .. } => *span,
            Self::IndexAccess { span, .. } => *span,
            Self::Assign { span, .. } => *span,
            Self::AssignFrom { span, .. } => *span,
            Self::LogicChain { span, .. } => *span,
            Self::If { span, .. } => *span,
            Self::Return(_, span) => *span,
            Self::Yield(_, span) => *span,
            Self::Block(_, span) => *span,
            Self::Await(_, span) => *span,
            Self::TyNamed(ident) => ident.1,
            Self::TyAccess { span, .. } => *span,
            Self::TyFunction { span, .. } => *span,
            Self::TyArray { span, .. } => *span,
            Self::TyTuple { span, .. } => *span,
            Self::TyOptional { span, .. } => *span,
            Self::TyRef { span, .. } => *span,
            Self::TyJoin { span, .. } => *span,
            Self::TypeOf { span, .. } => *span,
        }
    }
    #[cfg(test)]
    pub fn add_into(self, codebase: &mut Codebase) -> ExprId {
        codebase.exprs.add(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParseArgs {
    // Useful for tests
    pub allow_non_definitions_at_root: bool,
    pub add_std_prelude_import: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ParseArgs {
    fn default() -> Self {
        Self {
            allow_non_definitions_at_root: false,
            add_std_prelude_import: true,
        }
    }
}

#[derive(Debug)]
pub struct Ast(Vec<ExprId>);
impl Ast {
    pub fn parse(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> Ast {
        let first_span = tokens.last_span();
        let mut exprs = Expr::parse_semicolon_expr_list(tokens, codebase, !args.allow_non_definitions_at_root, args);
        
        // Add import for `std::prelude::{ ... }`
        if args.add_std_prelude_import {
            let import_path = IdentPath::from_components(&["std", "prelude"], first_span, codebase);
            exprs.insert(0, codebase.exprs.add(Expr::Using {
                visibility: Visibility::Private,
                path: UsingIdentPath {
                    parent: import_path,
                    item: UsingIdentItem::AllItems,
                    span: first_span,
                },
                span: first_span,
            }));
        }
        Ast(exprs)
    }
    pub fn exprs(&self) -> &[ExprId] {
        &self.0
    }
}

#[test]
fn invalid_parses() {
    let test_expr = |data: &str| {
        let (mut codebase, id) = Codebase::new_with_test_package("invalid_parses", data);
        codebase.parse_all(ParseArgs {
            allow_non_definitions_at_root: true,
            ..Default::default()
        });
        assert!(
            codebase.messages.counts().0 > 0,
            "`{data}` didn't result in errors:\n{}\ninstead got ast: {:#?}",
            codebase.messages.to_test_string(&codebase),
            codebase.parsed_asts.get(&id).unwrap()
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
    use crate::utils::tests::DebugAstEq;

    let (mut codebase, id) = Codebase::new_with_test_package("test_parse", r#"
        let x = 8;
        if x > 5 {
            x += lib::hi_guys();
        }
    "#);
    codebase.parse_all(ParseArgs {
        allow_non_definitions_at_root: true,
        add_std_prelude_import: false,
    });
    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty:\n{}", codebase.messages.to_test_string(&codebase)
    );

    let eq_ast = vec![
        Expr::Var {
            visibility: Visibility::Private,
            name: Ident(codebase.names.add("x"), Span::zero(id)),
            ty: None,
            value: Some(codebase.exprs.add(Expr::Int(8, Span::zero(id)))),
            span: Span::zero(id),
            is_const: false,
        }.add_into(&mut codebase),
        Expr::If {
            clause: Expr::CallOrTuple {
                target: Some(Expr::Ident(codebase.names.builtin_binop_name(Symbol::More, Span::zero(id))).add_into(&mut codebase)),
                args: vec![
                    (codebase.names.tuple_field(0, Span::zero(id)), Expr::Ident(IdentPath(
                        vec![Ident(codebase.names.add("x"), Span::zero(id))],
                        Span::zero(id)
                    )).add_into(&mut codebase)),
                    (codebase.names.tuple_field(1, Span::zero(id)), codebase.exprs.add(Expr::Int(5, Span::zero(id)))),
                ],
                op: Some((Symbol::More, Span::zero(id))),
                span: Span::zero(id)
            }.add_into(&mut codebase),
            truthy: Expr::Block(vec![
                Expr::Assign {
                    target: Expr::Ident(IdentPath(
                        vec![Ident(codebase.names.add("x"), Span::zero(id))],
                        Span::zero(id)
                    )).add_into(&mut codebase),
                    value: Expr::CallOrTuple {
                        target: Some(codebase.exprs.add(Expr::Ident(IdentPath(
                            vec![
                                Ident(codebase.names.add("lib"), Span::zero(id)),
                                Ident(codebase.names.add("hi_guys"), Span::zero(id)),
                            ],
                            Span::zero(id)
                        )))),
                        args: vec![],
                        op: None,
                        span: Span::zero(id)
                    }.add_into(&mut codebase),
                    op: (Symbol::AddAssign, Span::zero(id)),
                    span: Span::zero(id)
                }.add_into(&mut codebase)
            ], Span::zero(id)).add_into(&mut codebase),
            falsy: None,
            span: Span::zero(id)
        }.add_into(&mut codebase)
    ];

    let ast_exprs = &codebase.parsed_asts.get(&id).unwrap().0;
    assert_eq!(ast_exprs.len(), 2);
    ast_exprs.debug_ast_assert_eq(&eq_ast, &codebase);
}
