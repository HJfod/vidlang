use crate::{
    ast::expr::{Expr, Ident, ParseArgs, StructTypeField},
    codebase::Codebase,
    pools::{exprs::ExprId, messages::Message},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn parse_function_return_type(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> Option<ExprId> {
        if let Some((sym, span)) = tokens.peek_and_expect_symbol_of(codebase, |sym| matches!(sym, Symbol::Arrow | Symbol::FatArrow)) {
            if sym == Symbol::FatArrow {
                codebase.messages.add(Message::new_error(
                    "function types are defined with `->`, not `=>`",
                    span
                ));
            }
            return Some(Expr::parse_type(tokens, codebase, args));
        }
        None
    }
    fn parse_struct_type_field_inner(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> (Ident, Option<ExprId>) {
        let name = tokens.expect_ident(codebase);
        if tokens.peek_and_expect_symbol(Symbol::Colon, codebase) ||
            // Allow writing `field {}` shorthand
            tokens.peek_bracketed(BracketType::Braces, codebase)
        {
            let ty = Expr::parse_type(tokens, codebase, args);
            return (name, Some(ty));
        }
        (name, None)
    }
    fn parse_struct_type_field(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> StructTypeField {
        if tokens.peek_and_expect_symbol(Symbol::Enum, codebase) {
            let fields = match tokens.expect_bracketed(BracketType::Braces, codebase) {
                Token::Bracketed(_, mut fields, _) => Expr::parse_comma_list(
                    &mut fields, codebase, args, Expr::parse_struct_type_field_inner
                ),
                _ => vec![],
            };
            return StructTypeField::Enum(fields);
        }
        let (name, ty) = Expr::parse_struct_type_field_inner(tokens, codebase, args);
        let name_span = name.1;
        if ty.is_none() {
            codebase.messages.add(Message::new_error("non-enum fields must have an explicit type", name_span));
        }
        StructTypeField::Field(name, ty.unwrap_or_else(
            || codebase.exprs.add(Expr::Ident(codebase.names.missing_path(name_span)))
        ))
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

        // Object types `{ x: A, enum { y: B, z: C } }`
        if tokens.peek_bracketed(BracketType::Braces, codebase) {
            let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Braces, codebase) else {
                panic!("peek_bracketed returned true but expect_bracketed didn't");
            };
            let fields = Expr::parse_comma_list(&mut content, codebase, args, Expr::parse_struct_type_field);
            return codebase.exprs.add(Expr::TyObject { fields, span: tokens.span_from(start) });
        }

        // Function types `(A, B) -> C`
        if tokens.peek_bracketed(BracketType::Parentheses, codebase) &&
            tokens.peek_n(1).is_some_and(|s| matches!(s, Token::Symbol(Symbol::Arrow, _)))
        {
            let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Parentheses, codebase) else {
                panic!("peek_bracketed returned true but expect_bracketed didn't");
            };
            let params = Expr::parse_comma_list(&mut content, codebase, args, Expr::parse_type);
            let return_ty = match Expr::parse_function_return_type(tokens, codebase, args) {
                Some(ret) => ret,
                None => {
                    let span = tokens.last_span().next_ch();
                    codebase.messages.add(
                        Message::new_error("expected return type for function", span)
                            .with_note("Vid does not have unnamed tuple types; \
                                use objects with named variants instead", None)
                    );
                    codebase.exprs.add(Expr::TyNamed {
                        name: Ident(codebase.names.missing(), span),
                        span,
                    })
                }
            };
            return codebase.exprs.add(Expr::TyFunction {
                params,
                return_ty,
                span: tokens.span_from(start)
            });
        }

        // Parenthesized type expressions `(Thing)`
        if tokens.peek_bracketed(BracketType::Parentheses, codebase) {
            let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Parentheses, codebase) else {
                panic!("peek_bracketed returned true but expect_bracketed didn't");
            };
            return Expr::parse_type(&mut content, codebase, args);
        }

        // Typeof
        if tokens.peek_and_expect_symbol(Symbol::TypeOf, codebase) {
            let eval = Expr::parse(tokens, codebase, args);
            return codebase.exprs.add(Expr::TypeOf { eval, span: tokens.span_from(start) });
        }

        // Normal named type
        let name = tokens.expect_ident(codebase);
        codebase.exprs.add(Expr::TyNamed { name, span: tokens.span_from(start) })
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

            // Function types (yes these are parsed in two different places, 
            // but that's because I could not figure out any better way to 
            // support both `A -> B` and `(A, B) -> C`
            if let Some(return_ty) = Expr::parse_function_return_type(tokens, codebase, args) {
                inner = codebase.exprs.add(Expr::TyFunction {
                    params: vec![inner],
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
