use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, Ident, ParseArgs, StringComp},
    pools::{exprs::{ExprId, Exprs}, messages::Message},
    tokens::{token::{BracketType, StrLitComp, Symbol, Token},
    tokenstream::Tokens
}};

impl Expr {
    pub(super) fn parse_value(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs)
     -> ExprId
    {
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
            let start = tokens.start();

            let params = if tokens.peek_ident() {
                vec![FunctionParam {
                    kind: FunctionParamKind::Normal,
                    name: tokens.expect_ident(),
                    ty: None,
                    default_value: None
                }]
            }
            else {
                match tokens.expect_bracketed(BracketType::Parentheses) {
                    Token::Bracketed(_, mut params_tokens, _) => Expr::parse_comma_list(
                        Expr::parse_function_param, &mut params_tokens, exprs.clone(), args
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
            let body = Expr::parse(tokens, exprs.clone(), args);
            return exprs.add(Expr::ArrowFunction {
                params, body, span: tokens.span_from(start) }
            );
        }

        // Tuples
        if tokens.peek_bracketed(BracketType::Parentheses) {
            let start = tokens.start();
            let Token::Bracketed(_, mut sub_tokens, _) = tokens.expect_bracketed(BracketType::Parentheses) else {
                unreachable!("tokens.expect_bracketed didnt return Bracketed despite being peeked");
            };
            let fields = Expr::parse_comma_list(Expr::parse, &mut sub_tokens, exprs.clone(), args);
            return exprs.add(Expr::Tuple(fields, tokens.span_from(start)));
        }

        // Basic literals
        if tokens.peek_symbol(Symbol::False) || tokens.peek_symbol(Symbol::True) {
            let Some(Token::Symbol(sym, span)) = tokens.next() else {
                unreachable!("tokens.peek_symbol returned true but next() did not return a symbol");
            };
            return exprs.add(Expr::Bool(sym == Symbol::True, span));
        }
        if tokens.peek_int() {
            let Token::Int(num, span) = tokens.expect_int() else {
                unreachable!("tokens.peek_int() returned true but expect_int() did not return an integer");
            };
            return exprs.add(Expr::Int(num, span));
        }
        if tokens.peek_float() {
            let Token::Float(num, span) = tokens.expect_float() else {
                unreachable!("tokens.peek_float() returned true but expect_float() did not return a float");
            };
            return exprs.add(Expr::Float(num, span));
        }
        if tokens.peek_str() {
            let Token::String(value, span) = tokens.expect_str() else {
                unreachable!("tokens.peek_str() returned true but expect_str() did not return a string");
            };
            return exprs.add(Expr::String(
                value.into_iter().map(|c| match c {
                    StrLitComp::String(s) => StringComp::String(s),
                    StrLitComp::Component(mut c) => {
                        let expr = Expr::parse(&mut c, exprs.clone(), args);
                        c.expect_empty();
                        StringComp::Expr(expr)
                    }
                }).collect(),
                span
            ));
        }
        if tokens.peek_ident() {
            return exprs.add(Expr::Ident(tokens.expect_ident()));
        }

        let span = tokens.expected("expression");
        exprs.add(Expr::Ident(Ident(tokens.names().missing(), span)))
    }
}
