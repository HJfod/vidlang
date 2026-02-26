
use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, ParseArgs, TyExpr, Visibility},
    entities::messages::Message,
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_function_param(tokens: &mut Tokens, args: ParseArgs) -> FunctionParam {
        let kind;
        if tokens.peek_and_expect_symbol(Symbol::Ref) {
            kind = FunctionParamKind::Ref;
        }
        else if tokens.peek_and_expect_symbol(Symbol::Const) {
            kind = FunctionParamKind::Const;
        }
        else {
            kind = FunctionParamKind::Normal;
        }
        let name = tokens.expect_ident();
        let ty = tokens.peek_and_expect_symbol(Symbol::Colon)
            .then(|| TyExpr::parse(tokens, args));
        let default_value = tokens.peek_and_expect_symbol(Symbol::Assign)
            .then(|| Box::from(Expr::parse(tokens, args)));
        FunctionParam { kind, name, ty, default_value }
    }

    pub(super) fn try_parse_definition(tokens: &mut Tokens, args: ParseArgs) -> Option<Self> {
        let start = tokens.start();

        // Get visibility; by default, everything is private
        let mut visibility = Visibility::Private;
        let mut found_explicit_vis = None;
        while let Some((sym, span)) = tokens.peek_and_expect_symbol_of(
            |sym| matches!(sym, Symbol::Private | Symbol::Public)
        ) {
            if found_explicit_vis.is_some() {
                tokens.messages().add(Message::new_error(
                    "only one visibility modifier may be used per definition",
                    span
                ));
            }
            else {
                found_explicit_vis = Some(span);
                visibility = if sym == Symbol::Private { Visibility::Public } else { Visibility::Private };
            }
        }

        // Variable definition
        if let Some((sym, _)) = tokens.peek_and_expect_symbol_of(|sym| matches!(sym, Symbol::Let | Symbol::Const)) {
            let name = tokens.expect_ident();
            let ty = tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| TyExpr::parse(tokens, args));
            let value = tokens.peek_and_expect_symbol(Symbol::Assign)
                .then(|| Box::from(Expr::parse(tokens, args)));
            return Some(Expr::Var {
                visibility,
                name, ty, value,
                span: tokens.span_from(start),
                is_const: sym == Symbol::Const,
            });
        }

        // Function or clip definition
        if let Some((sym, _)) = tokens.peek_and_expect_symbol_of(
            |sym| matches!(sym, Symbol::Function | Symbol::Clip)
        ) {
            let name = tokens.expect_ident();
            let generics = TyExpr::try_parse_generic_params(tokens, args);
            let params = match tokens.expect_bracketed(BracketType::Parentheses) {
                Token::Bracketed(_, mut params_tokens, _) => Expr::parse_comma_list(
                    Expr::parse_function_param, &mut params_tokens, args
                ),
                _ => vec![],
            };
            let return_ty = tokens.peek_and_expect_symbol(Symbol::Arrow)
                .then(|| {
                    // Note: If I add object types this'll fail
                    if tokens.peek_bracketed(BracketType::Braces) {
                        tokens.messages().add(Message::expected_what(
                            "expected return type",
                            tokens.last_span()
                        ));
                        return None;
                    }
                    Some(TyExpr::parse(tokens, args))
                })
                .flatten();

            // Clips have special wacky types
            if let Some(ref ret) = return_ty && sym == Symbol::Clip {
                tokens.messages().add(Message::new_error(
                    "clips may not have explicit return types", ret.span()
                ));
            }

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
                visibility,
                name, generics, params, return_ty, body,
                is_clip: sym == Symbol::Clip,
                span: tokens.span_from(start)
            });
        }

        // If an explicit visibility specifier was used, then we know the user 
        // attempted to write a definition
        if let Some(span) = found_explicit_vis {
            // This purposefully doesn't consume, since this function returns 
            // an Option, so its caller will continue consuming. Not consuming 
            // here allows for possible error recovery
            tokens.messages().add(Message::expected_what("definition", span.next_ch()));
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
