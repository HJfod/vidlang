use crate::{ast::expr::{Expr, ParseArgs}, pools::{codebase::Codebase, exprs::ExprId}, tokens::{token::{BracketType, Symbol, Token}, tokenstream::Tokens}};

impl Expr {
    pub(super) fn parse_type(tokens: &mut Tokens, codebase: &mut Codebase, args: ParseArgs) -> ExprId {
        let start = tokens.start();

        // Array types `[Thing]`
        if tokens.peek_bracketed(BracketType::Brackets, codebase) {
            let inner = match tokens.expect_bracketed(BracketType::Brackets, codebase) {
                Token::Bracketed(_, mut content, _) => Expr::parse_type(&mut content, codebase, args),
                _ => codebase.exprs.add(Expr::Ident(codebase.names.missing_path(tokens.span_from(start)))),
            };
            return codebase.exprs.add(Expr::TyArray { inner, span: tokens.span_from(start) });
        }

        // Anytype (special thing for specific overloads)
        if tokens.peek_and_expect_symbol(Symbol::Anytype, codebase) {
            return codebase.exprs.add(Expr::TyAny(tokens.span_from(start)));
        }

        // Normal named type
        let name = Expr::parse_ident_path(tokens, codebase, args);
        codebase.exprs.add(Expr::TyNamed { name, span: tokens.span_from(start) })
    }
}

#[test]
fn type_parse() {
    use crate::pools::codebase::Codebase;
    use crate::ast::expr::ParseArgs;

    let (mut codebase, _) = Codebase::new_with_test_package("test_type_parse", r#"
        let x: A::B::C;
        let y: [string];
        let z: anytype;
    "#);
    codebase.parse_all(ParseArgs {
        allow_non_definitions_at_root: true
    });
    assert_eq!(
        codebase.messages.count_total(), 0,
        "messages was not empty:\n{}", codebase.messages.to_test_string(&codebase)
    );
}
