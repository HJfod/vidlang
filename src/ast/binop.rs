
use crate::{
    ast::expr::{Expr, Ident, IdentPath, LogicChainType, ParseArgs},
    codebase::Codebase,
    pools::{exprs::ExprId, messages::Message, modules::Span},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn make_binop_expr(codebase: &mut Codebase, op: Symbol, op_span: Span, lhs: ExprId, rhs: ExprId, full_span: Span) -> ExprId {
        let func_name = codebase.names.builtin_binop_name(op, op_span);
        let call = Expr::CallOrTuple {
            target: Some(codebase.exprs.add(Expr::Ident(func_name))),
            args: vec![
                (codebase.names.tuple_field(0, codebase.exprs.get(lhs).span()), lhs),
                (codebase.names.tuple_field(1, codebase.exprs.get(rhs).span()), rhs),
            ],
            op: Some((op, op_span)),
            span: full_span,
        };
        codebase.exprs.add(call)
    }

    pub(super) fn parse_from_expr(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs)
     -> (Vec<Ident>, ExprId)
    {
        tokens.expect_symbol(Symbol::From, codebase);
        let mut idents = vec![];
        // Remember to check for EOF here
        while tokens.peek().is_some() && !tokens.peek_bracketed(BracketType::Braces, codebase) {
            // Require identifiers to be separated with commas
            if !idents.is_empty() {
                tokens.expect_symbol(Symbol::Comma, codebase);
            }
            // Allow trailing comma
            if tokens.peek_bracketed(BracketType::Braces, codebase) {
                break;
            }
            idents.push(tokens.expect_ident(codebase));
        }
        // Check if the user forgot to add any idents, aka wrote `from { .. }`
        if idents.is_empty() {
            codebase.messages.add(Message::new_error(
                "from-expressions must depend on at least one property",
                tokens.last_span()
            ));
        }
        let body = Expr::parse_block(tokens, codebase, args);
        (idents, body)
    }

    fn parse_base(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        if let Some(cf) = Expr::try_parse_control_flow(tokens, codebase, args) {
            return cf;
        }
        if let Some(d) = Expr::try_parse_definition(tokens, codebase, args) {
            codebase.messages.add(Message::new_error(
                "definitions are not allowed here",
                codebase.exprs.get(d).span()
            ));
            return d;
        }
        // Check for false from expressions (`from <ident>` or `from { .. }`)
        if tokens.peek_symbol(Symbol::From, codebase) && tokens.peek_n(1).is_some_and(
            |t| matches!(t, Token::Ident(..) | Token::Bracketed(BracketType::Braces, ..))
        ) {
            let start = tokens.start();
            let (_, body) = Expr::parse_from_expr(tokens, codebase, args);
            codebase.messages.add(Message::new_error(
                "from-expressions are only allowed as the right-hand-side of assignments",
                tokens.span_from(start)
            ));
            return body;
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
            // Calls
            if tokens.peek_bracketed(BracketType::Parentheses, codebase) {
                let Token::Bracketed(_, mut args_tokens, _) = 
                    tokens.expect_bracketed(BracketType::Parentheses, codebase)
                else {
                    unreachable!("tokens.peek_bracketed returned true but expect_bracketed did not return Bracketed");
                };
                let mut args_counter = 0;
                let args = Expr::parse_comma_list(&mut args_tokens, codebase, args, |tks, pg, ag| {
                    Expr::parse_tuple_field(tks, pg, ag, &mut args_counter)
                });
                expr = codebase.exprs.add(Expr::CallOrTuple {
                    target: Some(expr),
                    args,
                    op: None,
                    span: tokens.span_from(start),
                });
                continue;
            }
            
            // Indexes
            if tokens.peek_bracketed(BracketType::Brackets, codebase) {
                let Token::Bracketed(_, mut args_tokens, _) = 
                    tokens.expect_bracketed(BracketType::Brackets, codebase)
                else {
                    unreachable!("tokens.peek_bracketed returned true but expect_bracketed did not return Bracketed");
                };
                let index = Expr::parse_expr(&mut args_tokens, codebase, args);
                args_tokens.expect_empty(codebase);
                expr = codebase.exprs.add(Expr::IndexAccess {
                    target: expr,
                    index,
                    span: tokens.span_from(start),
                });
                continue;
            }

            // Field accesses
            if let Some((sym, _)) = tokens.peek_and_expect_symbol_of(
                codebase, |t| matches!(t, Symbol::Dot | Symbol::QuestionDot)
            ) {
                let field_name = Expr::parse_ident_path(tokens, codebase, args);
                expr = codebase.exprs.add(Expr::FieldAccess {
                    target: expr,
                    field: field_name,
                    optional: sym == Symbol::QuestionDot, 
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
            let call = Expr::CallOrTuple {
                args: vec![(codebase.names.tuple_field(0, codebase.exprs.get(expr).span()), expr)],
                op: Some((op, func_name.1)),
                span: tokens.span_from(func_name.1.start()),
                target: Some(codebase.exprs.add(Expr::Ident(func_name))),
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
            if let Expr::CallOrTuple { args, op: Some((_, prev_span)), .. } = codebase.exprs.get(lhs) {
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

            lhs = Expr::make_binop_expr(codebase, op, span, lhs, rhs, tokens.span_from(start));
        }
        lhs
    }
    fn parse_binop_mul(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_power(tokens, codebase, args);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(
            codebase, |sym| matches!(sym, Symbol::Mul | Symbol::Div | Symbol::Mod)
        ) {
            let rhs = Expr::parse_binop_power(tokens, codebase, args);
            lhs = Expr::make_binop_expr(codebase, op, span, lhs, rhs, tokens.span_from(start));
        }
        lhs
    }
    fn parse_binop_sum(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_mul(tokens, codebase, args);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(
            codebase, |sym| matches!(sym, Symbol::Plus | Symbol::Minus)
        ) {
            let rhs = Expr::parse_binop_mul(tokens, codebase, args);
            lhs = Expr::make_binop_expr(codebase, op, span, lhs, rhs, tokens.span_from(start));
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
                lhs = Expr::make_binop_expr(codebase, op, span, lhs, rhs, tokens.span_from(start));
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
                let n = Expr::parse_binop_eq(tokens, codebase, args);
                values.push(n);
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
            found_one = true;

            if tokens.peek_symbol(Symbol::From, codebase) {
                let (properties, body) = Expr::parse_from_expr(tokens, codebase, args);
                lhs = codebase.exprs.add(Expr::AssignFrom {
                    target: lhs,
                    properties,
                    body,
                    span
                });
            }
            else {
                let rhs = Expr::parse_logic_chain(tokens, codebase, args);
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
            ..Default::default()
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
        add_std_prelude_import: false,
    });

    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty:\n{}", codebase.messages.to_test_string(&codebase)
    );

    let make_shit_up = |op: Symbol, lhs: ExprId, rhs: ExprId, codebase: &mut Codebase| {
        Expr::make_binop_expr(codebase, op, Span::zero(id), lhs, rhs, Span::zero(id))
    };

    // I live in väldigt sad
    let binop_tree = make_shit_up(Symbol::Plus,
        make_shit_up(Symbol::Minus,
            make_shit_up(Symbol::Plus,
                codebase.exprs.add(Expr::Int(1, Span::zero(id))),
                make_shit_up(Symbol::Mul,
                    codebase.exprs.add(Expr::Int(2, Span::zero(id))),
                    make_shit_up(Symbol::Power,
                        codebase.exprs.add(Expr::Int(3, Span::zero(id))),
                        codebase.exprs.add(Expr::Int(4, Span::zero(id))),
                        &mut codebase,
                    ),
                    &mut codebase,
                ),
                &mut codebase,
            ),
            codebase.exprs.add(Expr::Int(5, Span::zero(id))),
            &mut codebase,
        ),
        codebase.exprs.add(Expr::Int(6, Span::zero(id))),
        &mut codebase,
    );

    let compare_against = Vec::from([codebase.exprs.add(Expr::Yield(binop_tree, Span::zero(id)))]);
    let Expr::Ast { exprs: ast, .. } = codebase.exprs.get(*codebase.parsed_asts.get(&id).unwrap()) else {
        unreachable!();
    };
    assert_eq!(ast.len(), 1, "ast: {ast:?}");
    ast.debug_ast_assert_eq(&compare_against, &codebase);
}
