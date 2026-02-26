
use crate::{
    ast::expr::{Expr, Ident, ParseArgs, TyExpr},
    entities::messages::Message,
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    fn parse_function_arg(tokens: &mut Tokens, args: ParseArgs) -> (Ident, TyExpr, Option<Expr>) {
        let name = tokens.expect_ident();
        tokens.expect_symbol(Symbol::Colon);
        let ty = TyExpr::parse(tokens, args);
        let default_value = tokens.peek_and_expect_symbol(Symbol::Assign)
            .then(|| Expr::parse(tokens, args));
        (name, ty, default_value)
    }
    fn parse_arrow_function_arg(tokens: &mut Tokens, args: ParseArgs) -> (Ident, Option<TyExpr>, Option<Expr>) {
        let name = tokens.expect_ident();
        let ty = tokens.peek_and_expect_symbol(Symbol::Colon)
            .then(|| TyExpr::parse(tokens, args));
        let default_value = tokens.peek_and_expect_symbol(Symbol::Assign)
            .then(|| Expr::parse(tokens, args));
        (name, ty, default_value)
    }
    pub(super) fn try_parse_definition(tokens: &mut Tokens, args: ParseArgs) -> Option<Self> {
        let start = tokens.start();

        // Variable definition
        if let Some((sym, _)) = tokens.peek_and_expect_symbol_of(|sym| matches!(sym, Symbol::Let | Symbol::Const)) {
            let name = tokens.expect_ident();
            let ty = tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| TyExpr::parse(tokens, args));
            let value = tokens.peek_and_expect_symbol(Symbol::Assign)
                .then(|| Box::from(Expr::parse(tokens, args)));
            return Some(Expr::Var {
                name, ty, value,
                span: tokens.span_from(start),
                is_const: sym == Symbol::Const,
            });
        }

        // Function definition
        if tokens.peek_and_expect_symbol(Symbol::Function) {
            let name = tokens.expect_ident();
            let generics = TyExpr::try_parse_generic_params(tokens, args);
            let params = match tokens.expect_bracketed(BracketType::Parentheses) {
                Token::Bracketed(_, mut params_tokens, _) => Expr::parse_comma_list(
                    Expr::parse_function_arg, &mut params_tokens, args
                ),
                _ => vec![],
            };
            let return_ty = tokens.peek_and_expect_symbol(Symbol::Arrow)
                .then(|| TyExpr::parse(tokens, args));
            // Shorthand syntax `f() => expr`
            let body = Box::from(
                if tokens.peek_and_expect_symbol(Symbol::FatArrow) {
                    Expr::parse(tokens, args)
                }
                else {
                    Expr::parse_block(tokens, args)
                }
            );
            return Some(Expr::Function {
                name, generics, params, return_ty, body,
                span: tokens.span_from(start)
            });
        }

        // Arrow functions
        if (
            tokens.peek_bracketed(BracketType::Parentheses) || 
            // Allow `a => a` syntax
            tokens.peek_ident()
         ) &&
            tokens.peek_n(1).is_some_and(
                |t| matches!(t, Token::Symbol(Symbol::FatArrow | Symbol::Arrow, _))
            )
        {
            let params = if tokens.peek_ident() {
                vec![(tokens.expect_ident(), None, None)]
            }
            else {
                match tokens.expect_bracketed(BracketType::Parentheses) {
                    Token::Bracketed(_, mut params_tokens, _) => Expr::parse_comma_list(
                        Expr::parse_arrow_function_arg, &mut params_tokens, args
                    ),
                    _ => vec![],
                }
            };
            let Some(Token::Symbol(sym, sym_span)) = tokens.next() else {
                unreachable!("a symbol was previously peeked but tokens.next() did not return one");
            };
            if sym == Symbol::Arrow {
                tokens.messages().add(Message::new_error(
                    "arrow functions are defined with `=>`, not `->`",
                    sym_span
                ));
            }
            let body = Box::from(Expr::parse(tokens, args));
            return Some(Expr::ArrowFunction { params, body, span: tokens.span_from(start) });
        }
        
        None
    }
    pub(super) fn parse_definition(tokens: &mut Tokens, args: ParseArgs) -> Self {
        match Self::try_parse_definition(tokens, args) {
            Some(v) => v,
            None => {
                let start = tokens.start();
                // They probably tried to write an expr, so parsing one should 
                // result in less errors overall
                let bad_expr = Expr::parse(tokens, args);
                let span = tokens.span_from(start);
                tokens.messages().add(Message::new_error("only definitions may appear here", span));
                bad_expr
            }
        }
    }
}

#[test]
fn parse_arrow_function() {
    use crate::entities::codebase::Codebase;
    use crate::entities::names::Names;
    use crate::entities::messages::Messages;

    let mut codebase = Codebase::new();
    let names = Names::new();
    let messages = Messages::new();

    let _id = codebase.add_memory("parse_arrow_function", r#"
        let x = (a, b) => a + b;
        let y = a => a;
    "#);
    codebase.parse_all(names.clone(), messages.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });

    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );
}
