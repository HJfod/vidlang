
macro_rules! assert_ast {
    ($exprs: ident, $var: expr => $de: pat, $($rest: tt)*) => {
        $exprs.exec($var, |e| {
            let $de = e else { panic!() };
            assert_ast!($exprs, $($rest)*);
        });
    };
    ($exprs: ident, @run $run: expr, $($rest: tt)*) => {
        $run;
        assert_ast!($exprs, $($rest)*);
    };
    ($exprs: ident,) => {};
}

pub(crate) use assert_ast;
