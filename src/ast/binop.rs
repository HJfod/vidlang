
use crate::{
    ast::expr::{Expr, Ident, IdentPath, LogicChainType, ParseArgs},
    pools::{codebase::Codebase, exprs::ExprId, messages::Message},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn parse_call_arg(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> (Option<Ident>, ExprId) {
        if tokens.peek_ident(codebase) &&
            tokens.peek_n(1).is_some_and(
                |t| matches!(t, Token::Symbol(Symbol::Colon | Symbol::Assign, _))
            )
        {
            let ident = tokens.expect_ident(codebase);
            let Some(Token::Symbol(sym, sym_span)) = tokens.next() else {
                unreachable!("a symbol was previously peeked but tokens.next() did not return one");
            };
            if sym == Symbol::Assign {
                codebase.messages.add(Message::new_error(
                    "named function args are passed using `arg: value`, not with assignment",
                    sym_span
                ));
            }
            (Some(ident), Expr::parse(tokens, codebase, args))
        }
        else {
            (None, Expr::parse(tokens, codebase, args))
        }
    }

    fn parse_base(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        if let Some(cf) = Expr::try_parse_control_flow(tokens, codebase, args) {
            return cf;
        }
        if let Some(d) = Expr::try_parse_definition(tokens, codebase, args) {
            return d;
        }
        Expr::parse_value(tokens, codebase, args)
    }
    fn parse_unop(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        // Collect prefix unary operator (+, -, !)
        // Multiple operators are not allowed (since why would you ever 
        // actually write `--a`?)
        let mut unary_op: Option<(Symbol, IdentPath)> = None;
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(
            codebase,
            |sym| matches!(sym, Symbol::Plus | Symbol::Minus | Symbol::Exclamation)
        ) {
            if op == Symbol::Plus {
                codebase.messages.add(Message::new_error(
                    "unary plus operator is not supported",
                    span
                ));
            }
            else if unary_op.is_some() {
                codebase.messages.add(
                    Message::new_error(
                        "only one unary operator may be used at once",
                        span
                    ).with_hint("surround this operation in parentheses", Some(span))
                    // This epic code checks if the user tried to write `++` 
                    // or `--` C-style increment/decrement operators and issues 
                    // a note saying that those aren't supported if so
                    // (increment/decrement operators were an epic mistake)
                    .with_note_if(|| {
                        if unary_op.as_ref().is_some_and(
                            |u| u.0 == op &&
                            matches!(op, Symbol::Plus | Symbol::Minus)
                        ) {
                            return Some((
                                "C-style increment/decrement operators like '++' are not supported",
                                Some(span.extend_from(unary_op.as_ref().unwrap().1.1.start()))
                            ));
                        }
                        None
                    })
                );
            }
            else {
                let func_name = codebase.names.builtin_unop_name(op, span);
                unary_op = Some((op, func_name));
            }
        }

        // Parse an actual expression
        let start = tokens.start();
        let mut expr = Expr::parse_base(tokens, codebase, args);

        // Postfix unary operators (calls, indexes, etc.)
        loop {
            if tokens.peek_bracketed(BracketType::Parentheses, codebase) {
                let Token::Bracketed(_, mut args_tokens, _) = 
                    tokens.expect_bracketed(BracketType::Parentheses, codebase)
                else {
                    unreachable!("tokens.peek_bracketed returned true but expect_bracketed did not return Bracketed");
                };
                let args = Expr::parse_comma_list(&mut args_tokens, codebase, args, Expr::parse_call_arg);
                expr = codebase.exprs.add(Expr::Call {
                    target: expr,
                    args,
                    op: None,
                    span: tokens.span_from(start),
                });
                continue;
            }
            if tokens.peek_and_expect_symbol(Symbol::Dot, codebase) {
                let field_name = Expr::parse_ident_path(tokens, codebase, args);
                expr = codebase.exprs.add(Expr::FieldAccess {
                    target: expr,
                    field: field_name,
                    span: tokens.span_from(start)
                });
                continue;
            }
            break;
        }

        // Turn the previously parsed unary operator into a call expression
        // (this binds less tightly than the result of a suffix, since 
        // `-func() == -(func())`)
        if let Some((op, func_name)) = unary_op {
            let call = Expr::Call {
                args: vec![(None, expr)],
                op: Some((op, func_name.1)),
                span: tokens.span_from(func_name.1.start()),
                target: codebase.exprs.add(Expr::Ident(func_name)),
            };
            expr = codebase.exprs.add(call)
        }

        expr
    }
    fn parse_binop_power(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_unop(tokens, codebase, args);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(codebase, |sym| sym == Symbol::Power) {
            let rhs = Expr::parse_unop(tokens, codebase, args);

            // Check if LHS is an unary prefix operator, and issue a warning 
            // if so (since unary prefixes are ambiguous, as the mathematical 
            // parse for `-2 ** 5` would be `-(2 ** 5)`, but like, who is 
            // expecting that to happen)
            if let Expr::Call { args, op: Some((_, prev_span)), .. } = codebase.exprs.get(lhs) {
                // We differentiate between unary and binary operators based on 
                // argument count :-)
                if args.len() == 1 {
                    codebase.messages.add(
                        Message::new_error(
                            "unary prefix operators with power operators are ambiguous \
                            (`-a ** b` might be `-(a ** b)` or `(-a) ** b`)",
                            *prev_span
                        ).with_hint("add parentheses to resolve the ambiguity", None)
                    );
                }
            }

            let func_name = codebase.names.builtin_binop_name(op, span);
            let call = Expr::Call {
                target: codebase.exprs.add(Expr::Ident(func_name)),
                args: vec![(None, lhs), (None, rhs)],
                op: Some((op, span)),
                span: tokens.span_from(start)
            };
            lhs = codebase.exprs.add(call);
        }
        lhs
    }
    fn parse_binop_mul(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_power(tokens, codebase, args);
        let is_sum_sym = |sym| matches!(sym, Symbol::Mul | Symbol::Div | Symbol::Mod);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(codebase, is_sum_sym) {
            let rhs = Expr::parse_binop_power(tokens, codebase, args);
            let func_name = codebase.names.builtin_binop_name(op, span);
            let call = Expr::Call {
                target: codebase.exprs.add(Expr::Ident(func_name)),
                args: vec![(None, lhs), (None, rhs)],
                op: Some((op, span)),
                span: tokens.span_from(start)
            };
            lhs = codebase.exprs.add(call)
        }
        lhs
    }
    fn parse_binop_sum(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_mul(tokens, codebase, args);
        let is_sum_sym = |sym| matches!(sym, Symbol::Plus | Symbol::Minus);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(codebase, is_sum_sym) {
            let rhs = Expr::parse_binop_mul(tokens, codebase, args);
            let func_name = codebase.names.builtin_binop_name(op, span);
            let call = Expr::Call {
                target: codebase.exprs.add(Expr::Ident(func_name)),
                args: vec![(None, lhs), (None, rhs)],
                op: Some((op, span)),
                span: tokens.span_from(start)
            };
            lhs = codebase.exprs.add(call)
        }
        lhs
    }
    fn parse_binop_eq(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_sum(tokens, codebase, args);
        let is_eq_sym = |sym| matches!(sym, Symbol::Eq | Symbol::Neq | Symbol::Less | Symbol::Leq | Symbol::Meq | Symbol::More);

        let mut found_one = false;
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(codebase, is_eq_sym) {
            let rhs = Expr::parse_binop_sum(tokens, codebase, args);

            // We're only allowing one equality / comparison operator per binop 
            // (since `a == b > c` is honestly at best ambiguous and at worst 
            // hard-to-spot unintentional bug)
            // todo: allow (a == b == c) because that makes sense (and maybe (a < b < c))?
            if found_one {
                codebase.messages.add(
                    Message::new_error("only one comparison operator may be used at once", span)
                        .with_hint("surround this comparison in parentheses", None)
                );
            }
            else {
                found_one = true;
                let func_name = codebase.names.builtin_binop_name(op, span);
                let call = Expr::Call {
                    target: codebase.exprs.add(Expr::Ident(func_name)),
                    args: vec![(None, lhs), (None, rhs)],
                    op: Some((op, span)),
                    span: tokens.span_from(start)
                };
                lhs = codebase.exprs.add(call)
            }
        }
        lhs
    }
    /// Second lowest precedence binary operators: logic chains (`a and b and c` etc.)
    fn parse_logic_chain(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let first = Expr::parse_binop_eq(tokens, codebase, args);

        let is_logic_chain_sym = |sym| matches!(sym, Symbol::And | Symbol::Or);
        if let Some((orig_sym, _)) = tokens.peek_and_expect_symbol_of(codebase, is_logic_chain_sym) {
            let second = Expr::parse_binop_eq(tokens, codebase, args);
            let mut values = vec![first, second];

            // Parse the rest of the chain
            while let Some((next, span)) = tokens.peek_and_expect_symbol_of(codebase, is_logic_chain_sym) {
                values.push(Expr::parse_binop_eq(tokens, codebase, args));
                if next != orig_sym {
                    codebase.messages.add(
                        Message::new_error("mixing \"and\" and \"or\" expressions is ambiguous", span)
                            .with_hint("add parentheses to resolve the ambiguity", None)
                    );
                }
            }
            return codebase.exprs.add(Self::LogicChain {
                values,
                ty: if orig_sym == Symbol::And {
                    LogicChainType::And
                }
                else {
                    LogicChainType::Or
                },
                span: tokens.span_from(start)
            });
        }
        first
    }
    fn parse_binop_assign(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_logic_chain(tokens, codebase, args);
        let is_ass_sym = |sym| matches!(sym, Symbol::Assign | Symbol::AddAssign | Symbol::SubAssign);

        let mut found_one = false;
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(codebase, is_ass_sym) {
            let rhs = Expr::parse_logic_chain(tokens, codebase, args);

            if found_one {
                codebase.messages.add(
                    Message::new_error("only one assignment operator may be used at once", span)
                        .with_hint(
                            "surround this assignment in parentheses, or \
                            split the assigments into their own statements",
                            None
                        )
                );
            }
            else {
                found_one = true;
                lhs = codebase.exprs.add(Expr::Assign {
                    target: lhs,
                    value: rhs,
                    op: (op, span),
                    span: tokens.span_from(start)
                });
            }
        }
        lhs
    }
    pub(super) fn parse_binop(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        Self::parse_binop_assign(tokens, codebase, args)
    }
}

#[test]
fn ambiguous_exprs() {
    use crate::ast::expr::ParseArgs;

    let test_expr = |data: &str| {
        let (mut codebase, _) = Codebase::new_with_test_package("test_ambiguous_exprs", data);
        codebase.parse_all(ParseArgs {
            allow_non_definitions_at_root: true,
        });
        assert_eq!(
            codebase.messages.counts().0, 1,
            "`{data}` didn't result in one error:\n{}",
            codebase.messages.to_test_string(&codebase)
        );
    };

    test_expr("-2 ** 3");
    test_expr("a = b = c");
    test_expr("a == b < c");
    test_expr("a and b or c");
}

#[test]
fn binop() {
    use crate::pools::modules::Span;
    use crate::utils::tests::DebugAstEq;
    use crate::ast::expr::ParseArgs;

    let (mut codebase, id) = Codebase::new_with_test_package("test_binop", "1 + 2 * 3 ** 4 - 5 + 6");

    codebase.parse_all(ParseArgs {
        allow_non_definitions_at_root: true,
    });

    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty:\n{}", codebase.messages.to_test_string(&codebase)
    );

    let make_shit_up = |op: Symbol, lhs: Expr, rhs: Expr, codebase: &mut Codebase| {
        let target = codebase.exprs.add(Expr::Ident(codebase.names.builtin_binop_name(op, Span::zero(id))));
        let lhs = codebase.exprs.add(lhs);
        let rhs = codebase.exprs.add(rhs);
        Expr::Call {
            target,
            args: vec![(None, lhs), (None, rhs)],
            op: Some((op, Span::zero(id))),
            span: Span::zero(id)
        }
    };

    // I live in väldigt sad
    let binop_tree = make_shit_up(Symbol::Plus,
        make_shit_up(Symbol::Minus,
            make_shit_up(Symbol::Plus,
                Expr::Int(1, Span::zero(id)),
                make_shit_up(Symbol::Mul,
                    Expr::Int(2, Span::zero(id)),
                    make_shit_up(Symbol::Power,
                        Expr::Int(3, Span::zero(id)),
                        Expr::Int(4, Span::zero(id)),
                        &mut codebase,
                    ),
                    &mut codebase,
                ),
                &mut codebase,
            ),
            Expr::Int(5, Span::zero(id)),
            &mut codebase,
        ),
        Expr::Int(6, Span::zero(id)),
        &mut codebase,
    );
    let binop_tree = codebase.exprs.add(binop_tree);

    let ast = codebase.modules.get_ast_for(id).unwrap().exprs();
    assert_eq!(ast.len(), 1);
    ast.debug_ast_assert_eq(
        &[codebase.exprs.add(Expr::Yield(binop_tree, Span::zero(id)))],
        &codebase
    );
}
