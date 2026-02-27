use crate::{ast::expr::{Expr, Ident, ParseArgs}, pools::exprs::{ExprId, Exprs}, tokens::{token::Symbol, tokenstream::Tokens}};

impl Expr {
    pub(super) fn try_parse_generic_params(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs)
        -> Option<Vec<(Ident, Option<ExprId>)>>
    {
        if !tokens.peek_and_expect_symbol(Symbol::Less) {
            return None;
        }
        let mut generics = Vec::new();
        while tokens.peek().is_some() && !tokens.peek_symbol(Symbol::More) {
            let name = tokens.expect_ident();
            let constraint = tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| Expr::parse_type(tokens, exprs.clone(), args));
            generics.push((name, constraint));
            
            // This allows a trailing comma
            if tokens.peek_symbol(Symbol::More) {
                break;
            }
            Expr::parse_comma(tokens, exprs.clone(), args);
        }
        // This is separate to check for EOF
        tokens.expect_symbol(Symbol::More);
        Some(generics)
    }
    pub(super) fn try_parse_generic_args(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs)
        -> Option<Vec<ExprId>>
    {
        if !tokens.peek_and_expect_symbol(Symbol::Less) {
            return None;
        }
        let mut generics = Vec::new();
        while tokens.peek().is_some() && !tokens.peek_symbol(Symbol::More) {
            generics.push(Expr::parse_type(tokens, exprs.clone(), args));
            // This allows a trailing comma
            if tokens.peek_symbol(Symbol::More) {
                break;
            }
            Expr::parse_comma(tokens, exprs.clone(), args);
        }
        // This is separate to check for EOF
        tokens.expect_symbol(Symbol::More);
        Some(generics)
    }

    pub(super) fn parse_type(tokens: &mut Tokens, exprs: Exprs, args: ParseArgs) -> ExprId {
        let start = tokens.start();
        let name = tokens.expect_ident();
        let generics = Expr::try_parse_generic_args(tokens, exprs.clone(), args);
        let mut ty = exprs.add(Expr::TyNamed { name, generics, span: tokens.span_from(start) });

        // Associated types (`A<B>::C<D>`)
        while tokens.peek_and_expect_symbol(Symbol::Scope) {
            let associate = tokens.expect_ident();
            let generics = Expr::try_parse_generic_args(tokens, exprs.clone(), args);
            ty = exprs.add(Expr::TyAccess {
                from: ty,
                associate,
                generics,
                span: tokens.span_from(start)
            });
        }
        ty
    }
}

#[test]
fn type_parse() {
    use crate::pools::codebase::Codebase;
    use crate::pools::names::Names;
    use crate::pools::messages::Messages;

    let mut codebase = Codebase::new();
    let names = Names::new();
    let exprs = Exprs::new();
    let messages = Messages::new();

    let _id = codebase.add_memory("test_type_parse", r#"
        let x: A<B>::C<D, E>::F::G<H,>;
    "#);
    codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });
    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );
}
