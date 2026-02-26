use crate::{
    ast::expr::{Expr, Ident, LogicChainType},
    entities::{messages::Message, names::{NameId, Names}},
    tokens::{token::Symbol, tokenstream::Tokens}
};

fn get_builtin_func_name(symbol: Symbol, names: Names) -> Option<NameId> {
    match symbol {
        Symbol::Power => Some(names.add("op_power")),

        Symbol::Plus => Some(names.add("op_add")),
        Symbol::Minus => Some(names.add("op_sub")),
        Symbol::Mul => Some(names.add("op_mul")),
        Symbol::Div => Some(names.add("op_div")),
        Symbol::Mod => Some(names.add("op_mod")),

        Symbol::More => Some(names.add("op_more")),
        Symbol::Meq => Some(names.add("op_meq")),
        Symbol::Eq => Some(names.add("op_eq")),
        Symbol::Neq => Some(names.add("op_neq")),
        Symbol::Leq => Some(names.add("op_leq")),
        Symbol::Less => Some(names.add("op_less")),

        _ => None,
    }
}

impl Expr {
    fn parse_binop_mul(tokens: &mut Tokens) -> Self {}
    fn parse_binop_sum(tokens: &mut Tokens) -> Self {}
    fn parse_binop_eq(tokens: &mut Tokens) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_binop_sum(tokens);
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
                let rhs = Expr::parse_binop_sum(tokens);
                let func_name = get_builtin_func_name(op, tokens.names())
                    .expect(&format!("binary operator '{op}' did not have a builtin function name listed"));
                lhs = Expr::Call {
                    target: Box::from(Expr::Ident(Ident(func_name, span))),
                    args: vec![(None, lhs), (None, rhs)],
                    op: Some(op),
                    span: tokens.span_from(start)
                }
            }
        }
        lhs
    }
    /// Lowest precedence binary operators: logic chains (`a and b and c` etc.)
    fn parse_logic_chain(tokens: &mut Tokens) -> Self {
        let start = tokens.start();
        let first = Expr::parse_binop_eq(tokens);

        let is_logic_chain_sym = |sym| matches!(sym, Symbol::And | Symbol::Or);
        if let Some((orig_sym, _)) = tokens.peek_and_expect_symbol_of(is_logic_chain_sym) {
            let second = Expr::parse_binop_eq(tokens);
            let mut values = vec![first, second];

            // Parse the rest of the chain
            while let Some((next, span)) = tokens.peek_and_expect_symbol_of(is_logic_chain_sym) {
                values.push(Expr::parse_binop_eq(tokens));
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
    fn parse_binop_assign(tokens: &mut Tokens) -> Self {
        let start = tokens.start();
        let mut lhs = Expr::parse_logic_chain(tokens);
        let is_ass_sym = |sym| matches!(sym, Symbol::Assign | Symbol::AddAssign | Symbol::SubAssign);
        while let Some((op, _)) = tokens.peek_and_expect_symbol_of(is_ass_sym) {
            let rhs = Expr::parse_logic_chain(tokens);
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
                    op: Some(op),
                    span: tokens.span_from(start)
                }
            }
        }
        lhs
    }
    pub(super) fn parse_binop(tokens: &mut Tokens) -> Self {
        Self::parse_logic_chain(tokens)
    }
}
