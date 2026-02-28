use crate::{ast::expr::{Expr, ParseArgs}, pools::exprs::{ExprId, Exprs}, tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}};

impl Expr {
    pub(super) fn parse_type(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs) -> ExprId {
        let start = tokens.start();

        // Array types `[Thing]`
        if tokens.peek_bracketed(BracketType::Brackets) {
            let inner = match tokens.expect_bracketed(BracketType::Brackets) {
                Token::Bracketed(_, mut content, _) => Expr::parse_type(&mut content, exprs.clone(), args),
                _ => exprs.add(Expr::Ident(tokens.names().missing_path(tokens.span_from(start)))),
            };
            return exprs.add(Expr::TyArray { inner, span: tokens.span_from(start) });
        }

        // Anytype (special thing for specific overloads)
        if tokens.peek_and_expect_symbol(Symbol::Anytype) {
            return exprs.add(Expr::TyAny(tokens.span_from(start)));
        }

        // Normal named type
        let name = Expr::parse_ident_path(tokens, exprs.clone(), args);
        exprs.add(Expr::TyNamed { name, span: tokens.span_from(start) })
    }
}

#[test]
fn type_parse() {
    use crate::pools::codebase::Codebase;
    use crate::pools::names::Names;
    use crate::pools::messages::Messages;

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
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );
}
