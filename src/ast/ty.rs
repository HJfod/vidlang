use crate::{ast::expr::{Expr, Parser}, pools::exprs::ExprId, tokens::token::{BracketType, Symbol, Token}};

impl Expr {
    pub(super) fn parse_type(parser: &mut Parser) -> ExprId {
        let start = parser.tokens.start();

        // Array types `[Thing]`
        if parser.tokens.peek_bracketed(BracketType::Brackets) {
            let inner = match parser.tokens.expect_bracketed(BracketType::Brackets) {
                Token::Bracketed(_, mut content, _) => Expr::parse_type(&mut parser.fork(&mut content)),
                _ => parser.exprs.lock_mut().add(Expr::Ident(parser.tokens.names.lock_mut().missing_path(parser.tokens.span_from(start)))),
            };
            return parser.exprs.lock_mut().add(Expr::TyArray { inner, span: parser.tokens.span_from(start) });
        }

        // Anytype (special thing for specific overloads)
        if parser.tokens.peek_and_expect_symbol(Symbol::Anytype) {
            return parser.exprs.lock_mut().add(Expr::TyAny(parser.tokens.span_from(start)));
        }

        // Normal named type
        let name = Expr::parse_ident_path(parser);
        parser.exprs.lock_mut().add(Expr::TyNamed { name, span: parser.tokens.span_from(start) })
    }
}

#[test]
fn type_parse() {
    use crate::pools::codebase::Codebase;
    use crate::pools::names::Names;
    use crate::pools::messages::Messages;
    use crate::pools::exprs::Exprs;
    use crate::ast::expr::ParseArgs;

    let (mut codebase, _) = Codebase::new_with_test_package("test_type_parse", r#"
        let x: A::B::C;
        let y: [string];
        let z: anytype;
    "#);
    let names = Names::new();
    let exprs = Exprs::new();
    let messages = Messages::new();

    codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });
    assert_eq!(
        messages.lock().count_total(), 0,
        "messages was not empty:\n{}", messages.lock().to_test_string(&codebase)
    );
}
