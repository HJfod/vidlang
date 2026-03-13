use crate::{
    ast::expr::{Expr, Ident, IdentPath, ParseArgs, TupleTypeField},
    codebase::Codebase,
    pools::{exprs::ExprId, messages::Message},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn parse_tuple_type_field_inner(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs)
     -> (Ident, Option<ExprId>, Option<ExprId>)
    {
        // todo: unnamed fields
        let name = tokens.expect_ident(codebase);
        let ty = tokens.peek_and_expect_symbol(Symbol::Colon, codebase)
            .then(|| Expr::parse_type(tokens, codebase, args));
        let default_value = tokens.peek_and_expect_symbol(Symbol::Assign, codebase)
            .then(|| Expr::parse(tokens, codebase, args));
        (name, ty, default_value)
    }
    fn parse_tuple_type_field(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> TupleTypeField {
        if tokens.peek_and_expect_symbol(Symbol::Enum, codebase) {
            let fields = match tokens.expect_bracketed(BracketType::Braces, codebase) {
                Token::Bracketed(_, mut fields, _) => Expr::parse_comma_list(
                    &mut fields, codebase, args, |tks, cb, ag| {
                        let (name, ty, def) = Expr::parse_tuple_type_field_inner(tks, cb, ag);
                        if let Some(def) = def {
                            cb.messages.add(Message::new_error(
                                "enum fields may not have default values",
                                cb.exprs.get(def).span()
                            ));
                        }
                        (name, ty)
                    }
                ),
                _ => vec![],
            };
            return TupleTypeField::Enum(fields);
        }
        let (name, ty, def) = Expr::parse_tuple_type_field_inner(tokens, codebase, args);
        let name_span = name.1;
        if ty.is_none() {
            codebase.messages.add(Message::new_error("non-enum fields must have an explicit type", name_span));
        }
        TupleTypeField::Field(
            name,
            ty.unwrap_or_else(|| codebase.exprs.add(Expr::Ident(codebase.names.missing_path(name_span)))),
            def
        )
    }

    fn parse_type_inner(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();

        // Array types `[Thing]`
        if tokens.peek_bracketed(BracketType::Brackets, codebase) {
            let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Brackets, codebase) else {
                panic!("peek_bracketed returned true but expect_bracketed didn't");
            };
            let inner = Expr::parse_type(&mut content, codebase, args);
            return codebase.exprs.add(Expr::TyArray { inner, span: tokens.span_from(start) });
        }

        // Tuple types `(x: A, enum { y: B, z: C })` (and also function parameters!)
        if tokens.peek_bracketed(BracketType::Parentheses, codebase) {
            let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Parentheses, codebase) else {
                panic!("peek_bracketed returned true but expect_bracketed didn't");
            };
            let fields = Expr::parse_comma_list(&mut content, codebase, args, Expr::parse_tuple_type_field);
            return codebase.exprs.add(Expr::TyTuple { fields, span: tokens.span_from(start) });
        }

        // Typeof
        if tokens.peek_and_expect_symbol(Symbol::TypeOf, codebase) {
            let eval = Expr::parse(tokens, codebase, args);
            return codebase.exprs.add(Expr::TypeOf { eval, span: tokens.span_from(start) });
        }

        // Shorthand for clip and effect types via `clip name` syntax 
        // which is (shorthand for `name::Return`)
        if tokens.peek_and_expect_symbol_of(codebase, |sym| matches!(sym, Symbol::Clip | Symbol::Effect)).is_some() {
            let name = Expr::parse_ident_path(tokens, codebase, args);
            let span = name.1;
            let from = codebase.exprs.add(Expr::TyNamed(name));
            return codebase.exprs.add(Expr::TyAccess {
                from,
                item: IdentPath(vec![Ident(codebase.names.add("Return"), span)], span),
                span,
            });
        }

        // Normal named type
        let name = Expr::parse_ident_path(tokens, codebase, args);
        codebase.exprs.add(Expr::TyNamed(name))
    }

    pub(super) fn parse_type(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut inner = Expr::parse_type_inner(tokens, codebase, args);

        loop {
            // Optional types `Thing?`
            if tokens.peek_and_expect_symbol(Symbol::Question, codebase) {
                let mut found_additional_questions = false;
                let additional_questions_start = tokens.start();
                // Parse any additional question marks (which is ILLEGAL????)
                while tokens.peek_and_expect_symbol(Symbol::Question, codebase) {
                    found_additional_questions = true;
                }
                if found_additional_questions {
                    codebase.messages.add(Message::new_error(
                        "optional types may not be nested",
                        tokens.span_from(additional_questions_start)
                    ));
                }
                inner = codebase.exprs.add(Expr::TyOptional {
                    inner,
                    span: tokens.span_from(start)
                });
                continue;
            }

            // Accessed items `Thing::Other::Another`
            if tokens.peek_and_expect_symbol(Symbol::Scope, codebase) {
                let item = Expr::parse_ident_path(tokens, codebase, args);
                inner = codebase.exprs.add(Expr::TyAccess {
                    from: inner,
                    item,
                    span: tokens.span_from(start)
                });
                continue;
            }

            // Function types `A -> B`
            if let Some((sym, span)) = tokens.peek_and_expect_symbol_of(codebase, |sym| matches!(sym, Symbol::Arrow | Symbol::FatArrow)) {
                if sym == Symbol::FatArrow {
                    codebase.messages.add(Message::new_error(
                        "function types are defined with `->`, not `=>`",
                        span
                    ));
                }
                let return_ty = Expr::parse_type(tokens, codebase, args);
                inner = codebase.exprs.add(Expr::TyFunction {
                    param: inner,
                    return_ty,
                    span: tokens.span_from(start)
                });
                continue;
            }

            break;
        }

        // Otherwise just return inner
        inner
    }
}

#[test]
fn type_parse() {
    use crate::ast::expr::ParseArgs;
    let (mut codebase, _) = Codebase::new_with_test_package("test_type_parse", r#"
        let x: A::B::C;
        let y: [string];
        let z: S?;
        let a: typeof b;
        let b: (typeof y)::Item;
        let c: {
            x: int,
            y: float,
            z { carol: bool },
            enum {
                john,
                steve,
                caroline: int,
            }
        }
    "#);
    codebase.parse_all(ParseArgs {
        allow_non_definitions_at_root: true,
        ..Default::default()
    });
    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty:\n{}", codebase.messages.to_test_string(&codebase)
    );
}
