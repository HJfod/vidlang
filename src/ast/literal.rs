use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, IdentPath, ParseArgs, StringComp},
    pools::{codebase::Codebase, exprs::ExprId, messages::Message},
    tokens::{token::{BracketType, StrLitComp, Symbol, Token}, tokenstream::Tokens,
}};

impl Expr {
    pub(super) fn parse_ident_path(tokens: &mut Tokens, codebase: &mut Codebase, _args: ParseArgs) -> IdentPath {
        // All ident paths must have at least one ident to them
        let start = tokens.start();
        let mut res = vec![tokens.expect_ident(codebase)];
        while tokens.peek_and_expect_symbol(Symbol::Scope, codebase) {
            res.push(tokens.expect_ident(codebase));
        }
        IdentPath(res, tokens.span_from(start))
    }

    pub(super) fn parse_value(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        // Arrow functions
        if (
            tokens.peek_bracketed(BracketType::Parentheses, codebase) || 
            // Allow `a => a` syntax
            tokens.peek_ident(codebase)
         ) &&
            tokens.peek_n(1).is_some_and(
                |t| matches!(t, Token::Symbol(Symbol::FatArrow | Symbol::Arrow, _))
            )
        {
            let start = tokens.start();

            let params = if tokens.peek_ident(codebase) {
                vec![FunctionParam {
                    kind: FunctionParamKind::Normal,
                    name: tokens.expect_ident(codebase),
                    ty: None,
                    default_value: None
                }]
            }
            else {
                match tokens.expect_bracketed(BracketType::Parentheses, codebase) {
                    Token::Bracketed(_, mut params_tokens, _) => 
                        Expr::parse_comma_list(&mut params_tokens, codebase, args, Expr::parse_function_param),
                    _ => vec![],
                }
            };
            let Some(Token::Symbol(sym, sym_span)) = tokens.next() else {
                unreachable!("a symbol was previously peeked but tokens.next() did not return one");
            };
            if sym == Symbol::Arrow {
                codebase.messages.add(Message::new_error(
                    "arrow functions are defined with `=>`, not `->`",
                    sym_span
                ));
            }
            let body = Expr::parse(tokens, codebase, args);
            return codebase.exprs.add(Expr::ArrowFunction {
                params, body, span: tokens.span_from(start) }
            );
        }

        // Parenthesized expression `(something)`
        if tokens.peek_bracketed(BracketType::Parentheses, codebase) {
            let Token::Bracketed(_, mut sub_tokens, _) = tokens.expect_bracketed(BracketType::Parentheses, codebase) else {
                unreachable!("tokens.expect_bracketed didnt return Bracketed despite being peeked");
            };
            let content = Expr::parse(&mut sub_tokens, codebase, args);
            sub_tokens.expect_empty(codebase);
            return content;
        }

        // Basic literals
        if tokens.peek_symbol(Symbol::False, codebase) || tokens.peek_symbol(Symbol::True, codebase) {
            let Some(Token::Symbol(sym, span)) = tokens.next() else {
                unreachable!("tokens.peek_symbol returned true but next() did not return a symbol");
            };
            return codebase.exprs.add(Expr::Bool(sym == Symbol::True, span));
        }
        if tokens.peek_int(codebase) {
            let Token::Int(num, span) = tokens.expect_int(codebase) else {
                unreachable!("tokens.peek_int() returned true but expect_int() did not return an integer");
            };
            return codebase.exprs.add(Expr::Int(num, span));
        }
        if tokens.peek_float(codebase) {
            let Token::Float(num, span) = tokens.expect_float(codebase) else {
                unreachable!("tokens.peek_float() returned true but expect_float() did not return a float");
            };
            return codebase.exprs.add(Expr::Float(num, span));
        }
        if tokens.peek_duration(codebase) {
            let Token::Duration(num, span) = tokens.expect_duration(codebase) else {
                unreachable!("tokens.peek_duration() returned true but expect_duration() did not return a float");
            };
            return codebase.exprs.add(Expr::Duration(num, span));
        }
        if tokens.peek_str(codebase) {
            let Token::String(value, span) = tokens.expect_str(codebase) else {
                unreachable!("tokens.peek_str() returned true but expect_str() did not return a string");
            };
            let comps = value.into_iter().map(|c| match c {
                StrLitComp::String(s) => StringComp::String(s),
                StrLitComp::Component(mut c) => {
                    let expr = Expr::parse(&mut c, codebase, args);
                    c.expect_empty(codebase);
                    StringComp::Expr(expr)
                }
            }).collect();
            return codebase.exprs.add(Expr::String(comps, span));
        }
        if tokens.peek_ident(codebase) {
            let path = Expr::parse_ident_path(tokens, codebase, args);
            return codebase.exprs.add(Expr::Ident(path));
        }

        let span = tokens.expected("expression", codebase);
        codebase.exprs.add(Expr::Ident(codebase.names.missing_path(span)))
    }
}
