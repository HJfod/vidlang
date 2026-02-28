use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, IdentPath, Parser, StringComp},
    pools::{exprs::{ExprId}, messages::Message},
    tokens::{token::{BracketType, StrLitComp, Symbol, Token},
}};

impl Expr {
    pub(super) fn parse_ident_path(parser: &mut Parser<'_>) -> IdentPath {
        // All ident paths must have at least one ident to them
        let start = parser.tokens.start();
        let mut res = vec![parser.tokens.expect_ident()];
        while parser.tokens.peek_and_expect_symbol(Symbol::Scope) {
            res.push(parser.tokens.expect_ident());
        }
        IdentPath(res, parser.tokens.span_from(start))
    }

    pub(super) fn parse_value(parser: &mut Parser<'_>) -> ExprId {
        // Arrow functions
        if (
            parser.tokens.peek_bracketed(BracketType::Parentheses) || 
            // Allow `a => a` syntax
            parser.tokens.peek_ident()
         ) &&
            parser.tokens.peek_n(1).is_some_and(
                |t| matches!(t, Token::Symbol(Symbol::FatArrow | Symbol::Arrow, _))
            )
        {
            let start = parser.tokens.start();

            let params = if parser.tokens.peek_ident() {
                vec![FunctionParam {
                    kind: FunctionParamKind::Normal,
                    name: parser.tokens.expect_ident(),
                    ty: None,
                    default_value: None
                }]
            }
            else {
                match parser.tokens.expect_bracketed(BracketType::Parentheses) {
                    Token::Bracketed(_, mut params_tokens, _) => 
                        Expr::parse_comma_list(Expr::parse_function_param, &mut parser.fork(&mut params_tokens)),
                    _ => vec![],
                }
            };
            let Some(Token::Symbol(sym, sym_span)) = parser.tokens.next() else {
                unreachable!("a symbol was previously peeked but parser.tokens.next() did not return one");
            };
            if sym == Symbol::Arrow {
                parser.tokens.messages().add(Message::new_error(
                    "arrow functions are defined with `=>`, not `->`",
                    sym_span
                ));
            }
            let body = Expr::parse(parser);
            return parser.exprs.add(Expr::ArrowFunction {
                params, body, span: parser.tokens.span_from(start) }
            );
        }

        // Parenthesized expression `(something)`
        if parser.tokens.peek_bracketed(BracketType::Parentheses) {
            let Token::Bracketed(_, mut sub_tokens, _) = parser.tokens.expect_bracketed(BracketType::Parentheses) else {
                unreachable!("parser.tokens.expect_bracketed didnt return Bracketed despite being peeked");
            };
            let content = Expr::parse(&mut parser.fork(&mut sub_tokens));
            sub_tokens.expect_empty();
            return content;
        }

        // Basic literals
        if parser.tokens.peek_symbol(Symbol::False) || parser.tokens.peek_symbol(Symbol::True) {
            let Some(Token::Symbol(sym, span)) = parser.tokens.next() else {
                unreachable!("parser.tokens.peek_symbol returned true but next() did not return a symbol");
            };
            return parser.exprs.add(Expr::Bool(sym == Symbol::True, span));
        }
        if parser.tokens.peek_int() {
            let Token::Int(num, span) = parser.tokens.expect_int() else {
                unreachable!("parser.tokens.peek_int() returned true but expect_int() did not return an integer");
            };
            return parser.exprs.add(Expr::Int(num, span));
        }
        if parser.tokens.peek_float() {
            let Token::Float(num, span) = parser.tokens.expect_float() else {
                unreachable!("parser.tokens.peek_float() returned true but expect_float() did not return a float");
            };
            return parser.exprs.add(Expr::Float(num, span));
        }
        if parser.tokens.peek_duration() {
            let Token::Duration(num, span) = parser.tokens.expect_duration() else {
                unreachable!("parser.tokens.peek_duration() returned true but expect_duration() did not return a float");
            };
            return parser.exprs.add(Expr::Duration(num, span));
        }
        if parser.tokens.peek_str() {
            let Token::String(value, span) = parser.tokens.expect_str() else {
                unreachable!("parser.tokens.peek_str() returned true but expect_str() did not return a string");
            };
            return parser.exprs.add(Expr::String(
                value.into_iter().map(|c| match c {
                    StrLitComp::String(s) => StringComp::String(s),
                    StrLitComp::Component(mut c) => {
                        let expr = Expr::parse(&mut parser.fork(&mut c));
                        c.expect_empty();
                        StringComp::Expr(expr)
                    }
                }).collect(),
                span
            ));
        }
        if parser.tokens.peek_ident() {
            let path = Expr::parse_ident_path(parser);
            return parser.exprs.add(Expr::Ident(path));
        }

        let span = parser.tokens.expected("expression");
        parser.exprs.add(Expr::Ident(parser.tokens.names().missing_path(span)))
    }
}
