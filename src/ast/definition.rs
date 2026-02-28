
use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, Parser, Visibility},
    pools::{exprs::ExprId, messages::Message},
    tokens::token::{BracketType, Symbol, Token}
};

impl Expr {
    pub(super) fn parse_function_param(parser: &mut Parser<'_>)
     -> FunctionParam
    {
        let kind;
        if parser.tokens.peek_and_expect_symbol(Symbol::Ref) {
            kind = FunctionParamKind::Ref;
        }
        else if parser.tokens.peek_and_expect_symbol(Symbol::Const) {
            kind = FunctionParamKind::Const;
        }
        else {
            kind = FunctionParamKind::Normal;
        }
        let name = parser.tokens.expect_ident();
        let ty = parser.tokens.peek_and_expect_symbol(Symbol::Colon)
            .then(|| Expr::parse_type(parser));
        let default_value = parser.tokens.peek_and_expect_symbol(Symbol::Assign)
            .then(|| Expr::parse(parser));
        FunctionParam { kind, name, ty, default_value }
    }

    pub(super) fn try_parse_definition(parser: &mut Parser<'_>) -> Option<ExprId> {
        let start = parser.tokens.start();

        let is_vis_modifier = |sym| matches!(sym, Symbol::Private | Symbol::Public);

        // Get visibility; by default, everything is private
        let mut visibility = Visibility::Private;
        let mut found_explicit_vis = None;
        while let Some((sym, span)) = parser.tokens.peek_and_expect_symbol_of(is_vis_modifier) {
            if found_explicit_vis.is_some() {
                parser.tokens.messages().add(Message::new_error(
                    "only one visibility modifier may be used per definition",
                    span
                ));
            }
            else {
                found_explicit_vis = Some(span);
                visibility = if sym == Symbol::Private { Visibility::Public } else { Visibility::Private };
            }
        }

        let is_const = parser.tokens.peek_and_expect_symbol(Symbol::Const);
        let is_const_span = parser.tokens.last_span();

        // Check if there are visibility modifiers (aka wrong order)
        while let Some((_, span)) = parser.tokens.peek_and_expect_symbol_of(is_vis_modifier) {
            parser.tokens.messages().add(Message::new_error(
                "visibility modifiers must go before const specifier",
                span
            ));
        }

        // Modules
        if parser.tokens.peek_and_expect_symbol(Symbol::Module) {
            if is_const {
                parser.tokens.messages().add(Message::new_error(
                    "modules may not be marked const",
                    is_const_span,
                ));
            }
            let name = Expr::parse_ident_path(parser);
            let items = if let Token::Bracketed(_, mut content, _) = parser.tokens.expect_bracketed(BracketType::Braces) {
                Expr::parse_semicolon_expr_list(&mut parser.fork(&mut content), true)
            }
            else {
                Vec::new()
            };
            return Some(parser.exprs.add(Expr::Module { name, items, span: parser.tokens.span_from(start) }))
        }

        // Function or clip definition
        if let Some((sym, _)) = parser.tokens.peek_and_expect_symbol_of(
            |sym| matches!(sym, Symbol::Function | Symbol::Clip)
        ) {
            let name = Expr::parse_ident_path(parser);
            let params = match parser.tokens.expect_bracketed(BracketType::Parentheses) {
                Token::Bracketed(_, mut params_tokens, _) => 
                    Expr::parse_comma_list(Expr::parse_function_param, &mut parser.fork(&mut params_tokens)),
                _ => vec![],
            };
            let return_ty = parser.tokens.peek_and_expect_symbol(Symbol::Arrow)
                .then(|| {
                    // Note: If I add object types this'll fail
                    if parser.tokens.peek_bracketed(BracketType::Braces) {
                        parser.tokens.messages().add(Message::expected_what(
                            "expected return type",
                            parser.tokens.last_span()
                        ));
                        return None;
                    }
                    Some(Expr::parse(parser))
                })
                .flatten();

            // Clips have special wacky types
            if let Some(ref ret) = return_ty && sym == Symbol::Clip {
                parser.tokens.messages().add(Message::new_error(
                    "clips may not have explicit return types",
                    parser.exprs.exec(*ret, |e| e.span())
                ));
            }

            // Shorthand syntax `f() => expr`
            let body = if parser.tokens.peek_and_expect_symbol(Symbol::FatArrow) {
                Expr::parse(parser)
            }
            else {
                Expr::parse_block(parser)
            };
            return Some(parser.exprs.add(Expr::Function {
                visibility,
                name, params, return_ty, body,
                is_clip: sym == Symbol::Clip,
                is_const,
                span: parser.tokens.span_from(start)
            }));
        }

        // Variable definition (constants may be defined with shorthand 
        // `const a = x;`)
        if parser.tokens.peek_and_expect_symbol(Symbol::Let) || is_const {
            let name = parser.tokens.expect_ident();
            let ty = parser.tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| Expr::parse_type(parser));
            let value = parser.tokens.peek_and_expect_symbol(Symbol::Assign)
                .then(|| Expr::parse(parser));
            return Some(parser.exprs.add(Expr::Var {
                visibility,
                name, ty, value,
                span: parser.tokens.span_from(start),
                is_const,
            }));
        }

        // If an explicit visibility specifier was used, then we know the user 
        // attempted to write a definition
        if let Some(span) = found_explicit_vis {
            // This purposefully doesn't consume, since this function returns 
            // an Option, so its caller will continue consuming. Not consuming 
            // here allows for possible error recovery
            parser.tokens.messages().add(Message::expected_what("definition", span.next_ch()));
        }
        
        None
    }
    pub(super) fn parse_definition(parser: &mut Parser<'_>) -> ExprId {
        match Self::try_parse_definition(parser) {
            Some(v) => v,
            None => {
                let start = parser.tokens.start();
                // They probably tried to write an expr, so parsing one should 
                // result in less errors overall
                let bad_expr = Expr::parse(parser);
                let span = parser.tokens.span_from(start);
                parser.tokens.messages().add(Message::new_error("only definitions may appear here", span));
                bad_expr
            }
        }
    }
}

#[test]
fn parse_arrow_function() {
    use crate::pools::codebase::Codebase;
    use crate::pools::names::Names;
    use crate::pools::messages::Messages;
    use crate::pools::exprs::Exprs;
    use crate::ast::expr::ParseArgs;

    let (mut codebase, _) = Codebase::new_with_test_package("parse_arrow_function", r#"
        let x = (a, b) => a + b;
        let y = a => a;
    "#);

    let names = Names::new();
    let exprs = Exprs::new();
    let messages = Messages::new();
    codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });

    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );
}
