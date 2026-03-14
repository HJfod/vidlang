use crate::{
    ast::expr::{Expr, Ident, IdentPath, ParseArgs, TupleTypeField},
    codebase::Codebase,
    pools::{exprs::ExprId, messages::Message},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn parse_tuple_type_field(
        tokens: &mut Tokens,
        codebase: &mut Codebase,
        args: ParseArgs,
        field_counter: &mut usize
    ) -> TupleTypeField {
        if tokens.peek_and_expect_symbol(Symbol::Enum, codebase) {
            let fields = match tokens.expect_bracketed(BracketType::Braces, codebase) {
                Token::Bracketed(_, mut fields, _) => Expr::parse_comma_list(
                    &mut fields, codebase, args, |tks, cb, args| {
                        let name = tks.expect_ident(cb);
                        let ty = tks.peek_and_expect_symbol(Symbol::Colon, cb)
                            .then(|| Expr::parse_type(tks, cb, args));
                        if tks.peek_and_expect_symbol(Symbol::Assign, cb) {
                            let false_value = Expr::parse(tks, cb, args);
                            cb.messages.add(Message::new_error(
                                "enum fields may not have default values",
                                cb.exprs.get(false_value).span()
                            ));
                        }
                        (name, ty)
                    }
                ),
                _ => vec![],
            };
            return TupleTypeField::Enum(fields);
        }
        // If we're peeking `a: B` then parse name, otherwise parse unnamed field
        let is_const = tokens.peek_and_expect_symbol(Symbol::Const, codebase);
        let name;
        let ty;
        if tokens.peek_ident(codebase) && 
            tokens.peek_n(1).is_some_and(|t| matches!(t, Token::Symbol(Symbol::Colon, _)))
        {
            name = tokens.expect_ident(codebase);
            tokens.peek_and_expect_symbol(Symbol::Colon, codebase);
            ty = Expr::parse_type(tokens, codebase, args);
        }
        else {
            ty = Expr::parse_type(tokens, codebase, args);
            name = codebase.names.tuple_field(*field_counter, codebase.exprs.get(ty).span());
            *field_counter += 1;
        };
        let default = tokens.peek_and_expect_symbol(Symbol::Assign, codebase)
            .then(|| Expr::parse(tokens, codebase, args));
        TupleTypeField::Field { is_const, name, ty, default }
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
            let mut field_counter = 0;
            let fields = Expr::parse_comma_list(&mut content, codebase, args, |tks, cb, ag| {
                Expr::parse_tuple_type_field(tks, cb, ag, &mut field_counter)
            });
            return codebase.exprs.add(Expr::TyTuple { fields, span: tokens.span_from(start) });
        }

        // Shorthand for tuples with just one enum member
        if tokens.peek_symbol(Symbol::Enum, codebase) {
            let mut field_counter = 0;
            let field = Expr::parse_tuple_type_field(tokens, codebase, args, &mut field_counter);
            return codebase.exprs.add(Expr::TyTuple { fields: vec![field], span: tokens.span_from(start) });
        }

        // Typeof
        if tokens.peek_and_expect_symbol(Symbol::TypeOf, codebase) {
            let eval = Expr::parse(tokens, codebase, args);
            return codebase.exprs.add(Expr::TypeOf { eval, span: tokens.span_from(start) });
        }

        // Ref
        if tokens.peek_and_expect_symbol(Symbol::Ref, codebase) {
            let inner = Expr::parse_type(tokens, codebase, args);
            return codebase.exprs.add(Expr::TyRef { inner, span: tokens.span_from(start) });
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

            // Join types `A + B`
            if tokens.peek_and_expect_symbol(Symbol::Plus, codebase) {
                let rhs = Expr::parse_type(tokens, codebase, args);
                inner = codebase.exprs.add(Expr::TyJoin {
                    lhs: inner,
                    rhs,
                    span: tokens.span_from(start)
                });
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
        let c: (
            x: int,
            y: float,
            z: (carol: bool),
            enum {
                john,
                steve,
                caroline: int,
            }
        );
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
