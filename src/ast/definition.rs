
use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, FunctionType, IdentPath, ParseArgs, UsingIdentItem, UsingIdentPath, Visibility},
    codebase::Codebase,
    pools::{exprs::ExprId, messages::Message},
    tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}
};

impl Expr {
    pub(super) fn parse_function_param(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs)
     -> FunctionParam
    {
        let kind = if tokens.peek_and_expect_symbol(Symbol::Const, codebase) {
            FunctionParamKind::Const
        }
        else {
            FunctionParamKind::Normal
        };
        let name = tokens.expect_ident(codebase);
        let ty = tokens.peek_and_expect_symbol(Symbol::Colon, codebase)
            .then(|| Expr::parse_type(tokens, codebase, args));
        
        let mut from = vec![];
        if tokens.peek_and_expect_symbol(Symbol::From, codebase) {
            while tokens.peek_ident(codebase) {
                from.push(tokens.expect_ident(codebase));
                if !tokens.peek_and_expect_symbol(Symbol::Comma, codebase) {
                    break;
                }
            }
            if from.is_empty() {
                codebase.messages.add(Message::new_error(
                    "at least one dependent property name must be listed after the from-keyword",
                    tokens.last_span()
                ));
            }
        }

        let default_value = tokens.peek_and_expect_symbol(Symbol::Assign, codebase)
            .then(|| Expr::parse(tokens, codebase, args));
        FunctionParam { kind, name, ty, default_value, from }
    }

    fn parse_using_item_path(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> UsingIdentPath {
        let start = tokens.start();
        let mut parent_idents = vec![tokens.expect_ident(codebase)];

        // If there are no segments except the first one, that is an error
        if !tokens.peek_symbol(Symbol::Scope, codebase) {
            codebase.messages.add(Message::new_error("expected path to one or more items", tokens.span_from(start)));
            return UsingIdentPath {
                parent: IdentPath(parent_idents, tokens.span_from(start)),
                item: UsingIdentItem::Items(vec![]),
                span: tokens.span_from(start)
            };
        }

        while tokens.peek_and_expect_symbol(Symbol::Scope, codebase) {
            let span_up_to_this_scope = tokens.span_from(start);
            // Peeking any because we will assume all brackets are attempts to 
            // start importing multiple items
            if tokens.peek_any_bracketed(codebase) {
                let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Braces, codebase) else {
                    panic!("peek_any_brackected returned true but expect_bracketed did not return Bracketed");
                };
                let parent = IdentPath(parent_idents, span_up_to_this_scope);
                // Import everything with `{ ... }`
                if content.peek_and_expect_symbol(Symbol::DotDotDot, codebase) {
                    return UsingIdentPath {
                        parent,
                        item: UsingIdentItem::AllItems,
                        span: tokens.span_from(start),
                    };
                }
                // Otherwise assume there are a bunch of specific items being imported
                else {
                    let items = Expr::parse_comma_list(&mut content, codebase, args, |tks, cb, _| tks.expect_ident(cb));
                    return UsingIdentPath {
                        parent,
                        item: UsingIdentItem::Items(items),
                        span: tokens.span_from(start)
                    }
                }
            }
            else {
                let next_ident = tokens.expect_ident(codebase);
                // If this is the last segment, then we are importing just one item
                if !tokens.peek_symbol(Symbol::Scope, codebase) {
                    return UsingIdentPath {
                        parent: IdentPath(parent_idents, span_up_to_this_scope),
                        item: UsingIdentItem::Items(vec![next_ident]),
                        span: tokens.span_from(start)
                    }
                }
                // Otherwise push this to the parent list and keep going
                parent_idents.push(next_ident);
            }
        }

        unreachable!()
    }

    pub(super) fn try_parse_definition(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> Option<ExprId> {
        let start = tokens.start();

        let is_vis_modifier = |sym| matches!(sym, Symbol::Private | Symbol::Public);

        // Get visibility; by default, everything is public (except imports)
        let mut visibility = None;
        let mut found_explicit_vis = None;
        while let Some((sym, span)) = tokens.peek_and_expect_symbol_of(codebase, is_vis_modifier) {
            if found_explicit_vis.is_some() {
                codebase.messages.add(Message::new_error(
                    "only one visibility modifier may be used per definition",
                    span
                ));
            }
            else {
                found_explicit_vis = Some(span);
                visibility = Some(if sym == Symbol::Private { Visibility::Public } else { Visibility::Private });
            }
        }

        let is_const = tokens.peek_and_expect_symbol(Symbol::Const, codebase);
        let is_const_span = tokens.last_span();

        // Check if there are visibility modifiers (aka wrong order)
        while let Some((_, span)) = tokens.peek_and_expect_symbol_of(codebase, is_vis_modifier) {
            codebase.messages.add(Message::new_error(
                "visibility modifiers must go before const specifier",
                span
            ));
        }

        // Modules
        if tokens.peek_and_expect_symbol(Symbol::Module, codebase) {
            if is_const {
                codebase.messages.add(Message::new_error(
                    "modules may not be marked const",
                    is_const_span,
                ));
            }
            let name = Expr::parse_ident_path(tokens, codebase, args);
            let items = if let Token::Bracketed(_, mut content, _) = tokens.expect_bracketed(BracketType::Braces, codebase) {
                Expr::parse_semicolon_expr_list(&mut content, codebase, true, args)
            }
            else {
                Vec::new()
            };
            return Some(codebase.exprs.add(Expr::Module {
                visibility: visibility.unwrap_or(Visibility::Public),
                name,
                items,
                span: tokens.span_from(start)
            }))
        }

        // Function or clip definition
        if let Some((sym, _)) = tokens.peek_and_expect_symbol_of(
            codebase,
            |sym| matches!(sym, Symbol::Function | Symbol::Clip | Symbol::Effect)
        ) {
            let name = Expr::parse_ident_path(tokens, codebase, args);
            let params = match tokens.expect_bracketed(BracketType::Parentheses, codebase) {
                Token::Bracketed(_, mut params_tokens, _) => 
                    Expr::parse_comma_list(&mut params_tokens, codebase, args, Expr::parse_function_param),
                _ => vec![],
            };
            let return_ty = tokens.peek_and_expect_symbol(Symbol::Arrow, codebase)
                .then(|| {
                    // Note: If I add object types this'll fail
                    if tokens.peek_bracketed(BracketType::Braces, codebase) {
                        codebase.messages.add(Message::expected_what(
                            "expected return type",
                            tokens.last_span()
                        ));
                        return None;
                    }
                    Some(Expr::parse_type(tokens, codebase, args))
                })
                .flatten();

            // Clips and effects have special wacky types
            if let Some(ref ret) = return_ty && sym != Symbol::Function {
                codebase.messages.add(Message::new_error(
                    "clips and effects may not have explicit return types",
                    codebase.exprs.get(*ret).span()
                ));
            }

            let body = Expr::parse_block(tokens, codebase, args);
            return Some(codebase.exprs.add(Expr::Function {
                visibility: visibility.unwrap_or(Visibility::Public),
                ty: match sym {
                    Symbol::Function => FunctionType::Function,
                    Symbol::Clip => FunctionType::Clip,
                    Symbol::Effect => FunctionType::Effect,
                    _ => unreachable!(),
                },
                name, params, return_ty, body,
                is_const,
                span: tokens.span_from(start)
            }));
        }

        // Type definition `type A = B`
        if tokens.peek_and_expect_symbol(Symbol::Type, codebase) {
            if is_const {
                codebase.messages.add(Message::new_error(
                    "type definitions may not be marked const",
                    is_const_span
                ));
            }
            let name = Expr::parse_ident_path(tokens, codebase, args);
            tokens.peek_and_expect_symbol(Symbol::Assign, codebase);
            let ty = Expr::parse_type(tokens, codebase, args);
            return Some(codebase.exprs.add(Expr::TypeDef {
                visibility: visibility.unwrap_or(Visibility::Public),
                name,
                ty,
                span: tokens.span_from(start)
            }));
        }

        // Using definition `using A::{B, C}`
        if tokens.peek_and_expect_symbol(Symbol::Using, codebase) {
            if is_const {
                codebase.messages.add(Message::new_error(
                    "import declarations may not be marked const",
                    is_const_span
                ));
            }
            let path = Expr::parse_using_item_path(tokens, codebase, args);
            return Some(codebase.exprs.add(Expr::Using {
                visibility: visibility.unwrap_or(Visibility::Private),
                path,
                span: tokens.span_from(start)
            }));
        }

        // Variable definition (constants may be defined with shorthand 
        // `const a = x;`)
        if tokens.peek_and_expect_symbol(Symbol::Let, codebase) || is_const {
            let name = tokens.expect_ident(codebase);
            let ty = tokens.peek_and_expect_symbol(Symbol::Colon, codebase)
                .then(|| Expr::parse_type(tokens, codebase, args));
            let value = tokens.peek_and_expect_symbol(Symbol::Assign, codebase)
                .then(|| Expr::parse(tokens, codebase, args));
            return Some(codebase.exprs.add(Expr::Var {
                visibility: visibility.unwrap_or(if is_const { Visibility::Public } else { Visibility::Private }),
                name, ty, value,
                span: tokens.span_from(start),
                is_const,
            }));
        }

        // If an explicit visibility specifier was used, then we know the user 
        // attempted to write a definition
        if let Some(span) = found_explicit_vis {
            // This purposefully doesn't consume, since this function returns 
            // an Option, so its caller will continue consuming. Not consuming 
            // here allows for possible error recovery
            codebase.messages.add(Message::expected_what("definition", span.next_ch()));
        }
        
        None
    }
    pub(super) fn parse_definition(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        match Self::try_parse_definition(tokens, codebase, args) {
            Some(v) => v,
            None => {
                let start = tokens.start();
                // They probably tried to write an expr, so parsing one should 
                // result in less errors overall
                let bad_expr = Expr::parse(tokens, codebase, args);
                let span = tokens.span_from(start);
                codebase.messages.add(Message::new_error("only definitions may appear here", span));
                bad_expr
            }
        }
    }
}

#[test]
fn parse_arrow_function() {
    use crate::ast::expr::ParseArgs;

    let (mut codebase, _) = Codebase::new_with_test_package("parse_arrow_function", r#"
        let x = (a, b) -> a + b;
        let y = a -> a;
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
