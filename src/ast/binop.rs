use crate::{
    ast::expr::{Expr, Ident, LogicChainType, ParseArgs},
    entities::{codebase::Span, messages::Message, names::NameId},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn parse_call_arg(tokens: &mut Tokens, args: ParseArgs) -> (Option<Ident>, Expr) {
        if tokens.peek_ident() &&
            tokens.peek_n(1).is_some_and(
                |t| matches!(t, Token::Symbol(Symbol::Colon | Symbol::Assign, _))
            )
        {
            let ident = tokens.expect_ident();
            let Some(Token::Symbol(sym, sym_span)) = tokens.next() else {
                unreachable!("a symbol was previously peeked but tokens.next() did not return one");
            };
            if sym == Symbol::Assign {
                tokens.messages().add(Message::new_error(
                    "named function args are passed using `arg: value`, not with assignment",
                    sym_span
                ));
            }
            (Some(ident), Expr::parse(tokens, args))
        }
        else {
            (None, Expr::parse(tokens, args))
        }
    }

    fn parse_base(tokens: &mut Tokens, args: ParseArgs) -> Expr {
        if let Some(cf) = Expr::try_parse_control_flow(tokens, args) {
            return cf;
        }
        if let Some(d) = Expr::try_parse_definition(tokens, args) {
            return d;
        }
        Expr::parse_literal(tokens, args)
    }
    fn parse_unop(tokens: &mut Tokens, args: ParseArgs) -> Self {
        // Collect prefix unary operator (+, -, !)
        // Multiple operators are not allowed (since why would you ever 
        // actually write `--a`?)
        let mut unary_op: Option<(Symbol, NameId, Span)> = None;
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(
            |sym| matches!(sym, Symbol::Plus | Symbol::Minus | Symbol::Exclamation)
        ) {
            if op == Symbol::Plus {
                tokens.messages().add(Message::new_error(
                    "unary plus operator is not supported",
                    span
                ));
            }
            else if unary_op.is_some() {
                tokens.messages().add(
                    Message::new_error(
                        "only one unary operator may be used at once",
                        span
                    ).with_hint("surround this operation in parentheses", Some(span))
                    // This epic code checks if the user tried to write `++` 
                    // or `--` C-style increment/decrement operators and issues 
                    // a note saying that those aren't supported if so
                    // (increment/decrement operators were an epic mistake)
                    .with_note_if(|| {
                        if unary_op.is_some_and(
                            |u| u.0 == op &&
                            matches!(op, Symbol::Plus | Symbol::Minus)
                        ) {
                            return Some((
                                "C-style increment/decrement operators like '++' are not supported",
                                Some(span.extend_from(unary_op.unwrap().2.start()))
                            ));
                        }
                        None
                    })
                );
            }
            else {
                let func_name = tokens.names().builtin_unop_name(op);
                unary_op = Some((op, func_name, span));
            }
        }

        // Parse an actual expression
        let mut expr = Expr::parse_base(tokens, args);

        let start = expr.span().start();

        // Postfix unary operators (calls, indexes, etc.)
        loop {
            if tokens.peek_bracketed(BracketType::Parentheses) {
                let Token::Bracketed(_, mut args_tokens, _) = tokens.expect_bracketed(BracketType::Parentheses) else {
                    unreachable!("tokens.peek_bracketed returned true but expect_bracketed did not return Bracketed");
                };
                let args = Expr::parse_comma_list(Expr::parse_call_arg, &mut args_tokens, args);
                expr = Expr::Call {
                    target: Box::from(expr),
                    args,
                    op: None,
                    span: tokens.span_from(start),
                };
                continue;
            }
            if tokens.peek_and_expect_symbol(Symbol::Dot) {
                let field_name = tokens.expect_ident();
                expr = Expr::FieldAccess {
                    target: Box::from(expr),
                    field: field_name,
                    span: tokens.span_from(start)
                };
                continue;
            }
            break;
        }

        // Turn the previously parsed unary operator into a call expression
        // (this binds less tightly than the result of a suffix, since 
        // `-func() == -(func())`)
        if let Some((op, func_name, span)) = unary_op {
            expr = Expr::Call {
                target: Box::from(Expr::Ident(Ident(func_name, span))),
                args: vec![(None, expr)],
                op: Some((op, span)),
                span: tokens.span_from(span.start())
            }
        }

        expr
    }
    fn parse_binop_power(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_unop(tokens, args);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(|sym| sym == Symbol::Power) {
            let rhs = Expr::parse_unop(tokens, args);

            // Check if LHS is an unary prefix operator, and issue a warning 
            // if so (since unary prefixes are ambiguous, as the mathematical 
            // parse for `-2 ** 5` would be `-(2 ** 5)`, but like, who is 
            // expecting that to happen)
            if let Expr::Call { ref args, op: Some((_, prev_span)), .. } = lhs {
                // We differentiate between unary and binary operators based on 
                // argument count :-)
                if args.len() == 1 {
                    tokens.messages().add(
                        Message::new_error(
                            "unary prefix operators with power operators are ambiguous \
                            (`-a ** b` might be `-(a ** b)` or `(-a) ** b`)",
                            prev_span
                        ).with_hint("add parentheses to resolve the ambiguity", None)
                    );
                }
            } 

            let func_name = tokens.names().builtin_binop_name(op);
            lhs = Expr::Call {
                target: Box::from(Expr::Ident(Ident(func_name, span))),
                args: vec![(None, lhs), (None, rhs)],
                op: Some((op, span)),
                span: tokens.span_from(start)
            }
        }
        lhs
    }
    fn parse_binop_mul(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_power(tokens, args);
        let is_sum_sym = |sym| matches!(sym, Symbol::Mul | Symbol::Div | Symbol::Mod);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(is_sum_sym) {
            let rhs = Expr::parse_binop_power(tokens, args);
            let func_name = tokens.names().builtin_binop_name(op);
            lhs = Expr::Call {
                target: Box::from(Expr::Ident(Ident(func_name, span))),
                args: vec![(None, lhs), (None, rhs)],
                op: Some((op, span)),
                span: tokens.span_from(start)
            }
        }
        lhs
    }
    fn parse_binop_sum(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_mul(tokens, args);
        let is_sum_sym = |sym| matches!(sym, Symbol::Plus | Symbol::Minus);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(is_sum_sym) {
            let rhs = Expr::parse_binop_mul(tokens, args);
            let func_name = tokens.names().builtin_binop_name(op);
            lhs = Expr::Call {
                target: Box::from(Expr::Ident(Ident(func_name, span))),
                args: vec![(None, lhs), (None, rhs)],
                op: Some((op, span)),
                span: tokens.span_from(start)
            }
        }
        lhs
    }
    fn parse_binop_eq(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_sum(tokens, args);
        let is_eq_sym = |sym| matches!(sym, Symbol::Eq | Symbol::Neq | Symbol::Less | Symbol::Leq | Symbol::Meq | Symbol::More);

        let mut found_one = false;
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(is_eq_sym) {
            // We're only allowing one equality / comparison operator per binop 
            // (since `a == b > c` is honestly at best ambiguous and at worst 
            // hard-to-spot unintentional bug)
            // todo: allow (a == b == c) because that makes sense (and maybe (a < b < c))?
            if found_one {
                tokens.messages().add(
                    Message::new_error("only one comparison operator may be used at once", span)
                        .with_hint("surround this comparison in parentheses", None)
                );
            }
            else {
                found_one = true;
                let rhs = Expr::parse_binop_sum(tokens, args);
                let func_name = tokens.names().builtin_binop_name(op);
                lhs = Expr::Call {
                    target: Box::from(Expr::Ident(Ident(func_name, span))),
                    args: vec![(None, lhs), (None, rhs)],
                    op: Some((op, span)),
                    span: tokens.span_from(start)
                }
            }
        }
        lhs
    }
    /// Lowest precedence binary operators: logic chains (`a and b and c` etc.)
    fn parse_logic_chain(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let first = Expr::parse_binop_eq(tokens, args);

        let is_logic_chain_sym = |sym| matches!(sym, Symbol::And | Symbol::Or);
        if let Some((orig_sym, _)) = tokens.peek_and_expect_symbol_of(is_logic_chain_sym) {
            let second = Expr::parse_binop_eq(tokens, args);
            let mut values = vec![first, second];

            // Parse the rest of the chain
            while let Some((next, span)) = tokens.peek_and_expect_symbol_of(is_logic_chain_sym) {
                values.push(Expr::parse_binop_eq(tokens, args));
                if next != orig_sym {
                    tokens.messages().add(
                        Message::new_error("mixing \"and\" and \"or\" expressions is ambiguous", span)
                            .with_hint("add parentheses to resolve the ambiguity", None)
                    );
                }
            }
            return Self::LogicChain {
                values,
                ty: if orig_sym == Symbol::And {
                    LogicChainType::And
                }
                else {
                    LogicChainType::Or
                },
                span: tokens.span_from(start)
            };
        }
        first
    }
    fn parse_binop_assign(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_logic_chain(tokens, args);
        let is_ass_sym = |sym| matches!(sym, Symbol::Assign | Symbol::AddAssign | Symbol::SubAssign);
        while let Some((op, span)) = tokens.peek_and_expect_symbol_of(is_ass_sym) {
            let rhs = Expr::parse_logic_chain(tokens, args);
            // Assignment is right-associative
            if let Expr::Assign { target, value, op, span } = lhs {
                lhs = Expr::Assign {
                    target,
                    value: Box::from(Expr::Assign {
                        target: value,
                        value: rhs.into(),
                        op,
                        span
                    }),
                    op,
                    span
                }
            }
            else {
               lhs = Expr::Assign {
                    target: lhs.into(),
                    value: rhs.into(),
                    op: (op, span),
                    span: tokens.span_from(start)
                }
            }
        }
        lhs
    }
    pub(super) fn parse_binop(tokens: &mut Tokens, args: ParseArgs) -> Self {
        Self::parse_binop_assign(tokens, args)
    }
}

#[test]
fn test_binop() {
    use crate::entities::codebase::{Codebase, Span};
    use crate::entities::messages::Messages;
    use crate::entities::names::Names;

    let mut codebase = Codebase::new();
    let id = codebase.add_memory("test_binop", "1 + 2 * 3 ** 4 - 5 + 6");

    let names = Names::new();
    let messages = Messages::new();
    codebase.parse_all(names.clone(), messages.clone(), ParseArgs {
        allow_non_definitions_at_root: true,
    });

    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );

    let ast = codebase.fetch(id).ast().unwrap().exprs();
    assert_eq!(ast.len(), 1);

    let make_shit_up = |op: Symbol, lhs: Expr, rhs: Expr| Expr::Call {
        target: Box::from(Expr::Ident(Ident(
            names.builtin_binop_name(op),
            Span::zero(id)
        ))),
        args: vec![(None, lhs), (None, rhs)],
        op: Some((op, Span::zero(id))),
        span: Span::zero(id)
    };

    assert_eq!(ast[0],
        Expr::Yield(
            make_shit_up(Symbol::Plus, 
                make_shit_up(Symbol::Minus,
                    make_shit_up(Symbol::Plus, 
                        Expr::Int(1, Span::zero(id)),
                        make_shit_up(Symbol::Mul,
                            Expr::Int(2, Span::zero(id)),
                            make_shit_up(Symbol::Power,
                                Expr::Int(3, Span::zero(id)),
                                Expr::Int(4, Span::zero(id)),
                            ),
                        ),
                    ),
                    Expr::Int(5, Span::zero(id)),
                ),
                Expr::Int(6, Span::zero(id)),
            ).into(),
            Span::zero(id)
        )
    );
}
