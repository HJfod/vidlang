use crate::{ast::expr::{Expr, ParseArgs}, pools::exprs::{ExprId, Exprs}, tokens::tokenstream::Tokens};

impl Expr {
    pub(super) fn parse_type(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let name = Expr::parse_ident_path(tokens, exprs.clone(), args);
        exprs.add(Expr::TyNamed { name, span: tokens.span_from(start) })
    }
}

#[test]
fn type_parse() {
    use crate::pools::codebase::Codebase;
    use crate::pools::names::Names;
    use crate::pools::messages::Messages;

    let mut codebase = Codebase::from_memory("test_type_parse", r#"
        let x: A::B::C;
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
