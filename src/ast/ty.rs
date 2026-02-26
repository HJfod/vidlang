use crate::{ast::expr::{Expr, Ident, ParseArgs, TyExpr}, tokens::{token::Symbol, tokenstream::Tokens}};

impl TyExpr {
    pub(super) fn try_parse_generic_params(tokens: &mut Tokens, args: ParseArgs)
        -> Option<Vec<(Ident, Option<TyExpr>)>>
    {
        if !tokens.peek_and_expect_symbol(Symbol::Less) {
            return None;
        }
        let mut generics = Vec::new();
        while tokens.peek().is_some() && !tokens.peek_symbol(Symbol::More) {
            let name = tokens.expect_ident();
            let constraint = tokens.peek_and_expect_symbol(Symbol::Colon)
                .then(|| TyExpr::parse(tokens, args));
            generics.push((name, constraint));
            
            // This allows a trailing comma
            if tokens.peek_symbol(Symbol::More) {
                break;
            }
            Expr::parse_comma(tokens, args);
        }
        // This is separate to check for EOF
        tokens.expect_symbol(Symbol::More);
        Some(generics)
    }
    pub(super) fn try_parse_generic_args(tokens: &mut Tokens, args: ParseArgs) -> Option<Vec<TyExpr>> {
        if !tokens.peek_and_expect_symbol(Symbol::Less) {
            return None;
        }
        let mut generics = Vec::new();
        while tokens.peek().is_some() && !tokens.peek_symbol(Symbol::More) {
            generics.push(TyExpr::parse(tokens, args));
            // This allows a trailing comma
            if tokens.peek_symbol(Symbol::More) {
                break;
            }
            Expr::parse_comma(tokens, args);
        }
        // This is separate to check for EOF
        tokens.expect_symbol(Symbol::More);
        Some(generics)
    }

    pub(super) fn parse(tokens: &mut Tokens, args: ParseArgs) -> Self {
        let start = tokens.start();
        let name = tokens.expect_ident();
        let generics = TyExpr::try_parse_generic_args(tokens, args);
        let mut ty = TyExpr::Named { name, generics, span: tokens.span_from(start) };

        // Associated types (`A<B>::C<D>`)
        while tokens.peek_and_expect_symbol(Symbol::Scope) {
            let associate = tokens.expect_ident();
            let generics = TyExpr::try_parse_generic_args(tokens, args);
            ty = TyExpr::Access {
                from: Box::from(ty),
                associate,
                generics,
                span: tokens.span_from(start)
            };
        }

        ty
    }
}

#[test]
fn type_parse() {
    use crate::entities::codebase::Codebase;
    use crate::entities::names::Names;
    use crate::entities::messages::Messages;

    let mut codebase = Codebase::new();
    let names = Names::new();
    let messages = Messages::new();

    let _id = codebase.add_memory("test_type_parse", r#"
        let x: A<B>::C<D, E>::F::G<H,>;
    "#);
    codebase.parse_all(names.clone(), messages.clone(), ParseArgs {
        allow_non_definitions_at_root: true
    });
    assert_eq!(
        messages.count_total(), 0,
        "messages was not empty:\n{}", messages.to_test_string(&codebase)
    );
}
