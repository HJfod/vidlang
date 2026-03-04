
#![cfg(test)]

use crate::{
    ast::expr::{Expr, FunctionParam, FunctionParamKind, Ident, IdentPath, LogicChainType, StringComp, Visibility},
    pools::{exprs::ExprId, modules::Span, names::NameId},
    tokens::token::Symbol,
    codebase::Codebase
};

pub trait DebugAstEq {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase);
}

// It would be very neat if we could auto-derive impl for all PartialEq but 
// unfortunately ExprId is PartialEq and no specialization </3

impl DebugAstEq for bool {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(*self, *other)
    }
}
impl DebugAstEq for u64 {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(*self, *other)
    }
}
impl DebugAstEq for f64 {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(*self, *other)
    }
}
impl DebugAstEq for String {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(*self, *other)
    }
}

impl<A: DebugAstEq, B: DebugAstEq> DebugAstEq for (A, B) {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
       self.0.debug_ast_assert_eq(&other.0, codebase);
       self.1.debug_ast_assert_eq(&other.1, codebase);
    }
}
impl<T: DebugAstEq> DebugAstEq for Option<T> {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        assert_eq!(self.is_some(), other.is_some());
        if let Some(a) = self && let Some(b) = other {
            a.debug_ast_assert_eq(b, codebase);
        }
    }
}
impl<T: DebugAstEq> DebugAstEq for [T] {
    fn debug_ast_assert_eq(&self, other: &Self,codebase: &Codebase) {
        assert_eq!(self.len(), other.len());
        for (a, b) in self.iter().zip(other.iter()) {
            a.debug_ast_assert_eq(b, codebase);
        }
    }
}
impl<T: DebugAstEq> DebugAstEq for Vec<T> {
    fn debug_ast_assert_eq(&self, other: &Self,codebase: &Codebase) {
        assert_eq!(self.len(), other.len());
        for (a, b) in self.iter().zip(other.iter()) {
            a.debug_ast_assert_eq(b, codebase);
        }
    }
}

impl DebugAstEq for Span {
    fn debug_ast_assert_eq(&self, _other: &Self, _codebase: &Codebase) {
        // We don't care if spans are equal or not
    }
}
impl DebugAstEq for Symbol {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(self, other)
    }
}
impl DebugAstEq for NameId {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(self, other);
    }
}
impl DebugAstEq for ExprId {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        codebase.exprs.get(*self).debug_ast_assert_eq(codebase.exprs.get(*other), codebase);
    }
}

impl DebugAstEq for Ident {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        self.0.debug_ast_assert_eq(&other.0, codebase);
    }
}
impl DebugAstEq for IdentPath {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        self.0.debug_ast_assert_eq(&other.0, codebase);
    }
}
impl DebugAstEq for Visibility {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(std::mem::discriminant(self), std::mem::discriminant(other));
    }
}
impl DebugAstEq for FunctionParamKind {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(std::mem::discriminant(self), std::mem::discriminant(other));
    }
}
impl DebugAstEq for FunctionParam {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        self.name.debug_ast_assert_eq(&other.name, codebase);
        self.kind.debug_ast_assert_eq(&other.kind, codebase);
        self.ty.debug_ast_assert_eq(&other.ty, codebase);
        self.default_value.debug_ast_assert_eq(&other.default_value, codebase);
    }
}
impl DebugAstEq for LogicChainType {
    fn debug_ast_assert_eq(&self, other: &Self, _codebase: &Codebase) {
        assert_eq!(std::mem::discriminant(self), std::mem::discriminant(other));
    }
}
impl DebugAstEq for StringComp {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        match (self, other) {
            (StringComp::String(a), StringComp::String(b)) => a.debug_ast_assert_eq(b, codebase),
            (StringComp::Expr(a), StringComp::Expr(b)) => a.debug_ast_assert_eq(b, codebase),
            _ => panic!("StringComps weren't equal in debug_ast_assert_eq")
        }
    }
}

// This really should be derived but I don't feel like writing proc macros again
impl DebugAstEq for Expr {
    fn debug_ast_assert_eq(&self, other: &Self, codebase: &Codebase) {
        assert_eq!(std::mem::discriminant(self), std::mem::discriminant(other));
        match (self, other) {
            (Expr::Bool(a, _), Expr::Bool(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::Int(a, _), Expr::Int(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::Float(a, _), Expr::Float(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::String(a, _), Expr::String(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::Ident(a), Expr::Ident(b)) => a.debug_ast_assert_eq(b, codebase),

            (
                Expr::Var {
                    visibility: a_visibility,
                    name: a_name,
                    ty: a_ty,
                    value: a_value,
                    is_const: a_is_const,
                    span: _
                },
                Expr::Var {
                    visibility: b_visibility,
                    name: b_name,
                    ty: b_ty,
                    value: b_value,
                    is_const: b_is_const,
                    span: _
                },
            ) => {
                a_visibility.debug_ast_assert_eq(b_visibility, codebase);
                a_name.debug_ast_assert_eq(b_name, codebase);
                a_ty.debug_ast_assert_eq(b_ty, codebase);
                a_value.debug_ast_assert_eq(b_value, codebase);
                a_is_const.debug_ast_assert_eq(b_is_const, codebase);
            }
            (
                Expr::Function {
                    visibility: a_visibility,
                    name: a_name,
                    params: a_params,
                    return_ty: a_return_ty,
                    body: a_body,
                    is_clip: a_is_clip,
                    is_const: a_is_const,
                    span:_ 
                },
                Expr::Function {
                    visibility: b_visibility,
                    name: b_name,
                    params: b_params,
                    return_ty: b_return_ty,
                    body: b_body,
                    is_clip: b_is_clip,
                    is_const: b_is_const,
                    span:_ 
                },
            ) => {
                a_visibility.debug_ast_assert_eq(b_visibility, codebase);
                a_name.debug_ast_assert_eq(b_name, codebase);
                a_params.debug_ast_assert_eq(b_params, codebase);
                a_return_ty.debug_ast_assert_eq(b_return_ty, codebase);
                a_body.debug_ast_assert_eq(b_body, codebase);
                a_is_clip.debug_ast_assert_eq(b_is_clip, codebase);
                a_is_const.debug_ast_assert_eq(b_is_const, codebase);
            }
            (
                Expr::ArrowFunction {
                    params: a_params,
                    body: a_body,
                    span: _
                },
                Expr::ArrowFunction {
                    params: b_params,
                    body: b_body,
                    span: _
                },
            ) => {
                a_params.debug_ast_assert_eq(b_params, codebase);
                a_body.debug_ast_assert_eq(b_body, codebase);
            }

            (
                Expr::Call {
                    target: a_target,
                    args: a_args,
                    op: a_op,
                    span: _
                },
                Expr::Call {
                    target: b_target,
                    args: b_args,
                    op: b_op,
                    span: _
                },
            ) => {
                a_target.debug_ast_assert_eq(b_target, codebase);
                a_args.debug_ast_assert_eq(b_args, codebase);
                a_op.debug_ast_assert_eq(b_op, codebase);
            },
            (
                Expr::FieldAccess {
                    target: a_target,
                    field: a_field,
                    span: _,
                },
                Expr::FieldAccess {
                    target: b_target,
                    field: b_field,
                    span: _,
                },
            ) => {
                a_target.debug_ast_assert_eq(b_target, codebase);
                a_field.debug_ast_assert_eq(b_field, codebase);
            },
            (
                Expr::Assign {
                    target: a_target,
                    value: a_value,
                    op: a_op,
                    span: _,
                },
                Expr::Assign {
                    target: b_target,
                    value: b_value,
                    op: b_op,
                    span: _,
                },
            ) => {
                a_target.debug_ast_assert_eq(b_target, codebase);
                a_value.debug_ast_assert_eq(b_value, codebase);
                a_op.debug_ast_assert_eq(b_op, codebase);
            },
            (
                Expr::LogicChain {
                    values: a_values,
                    ty: a_ty,
                    span: _,
                },
                Expr::LogicChain {
                    values: b_values,
                    ty: b_ty,
                    span: _,
                },
            ) => {
                a_values.debug_ast_assert_eq(b_values, codebase);
                a_ty.debug_ast_assert_eq(b_ty, codebase);
            },
            (
                Expr::If {
                    clause: a_clause,
                    truthy: a_truthy,
                    falsy: a_falsy,
                    span: _,
                },
                Expr::If {
                    clause: b_clause,
                    truthy: b_truthy,
                    falsy: b_falsy,
                    span: _,
                },
            ) => {
                a_clause.debug_ast_assert_eq(b_clause, codebase);
                a_truthy.debug_ast_assert_eq(b_truthy, codebase);
                a_falsy.debug_ast_assert_eq(b_falsy, codebase);
            },
            (Expr::Return(a, _), Expr::Return(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::Yield(a, _), Expr::Yield(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::Block(a, _), Expr::Block(b, _)) => a.debug_ast_assert_eq(b, codebase),
            (Expr::Await(a, _), Expr::Await(b, _)) => a.debug_ast_assert_eq(b, codebase),

            (
                Expr::TyNamed {
                    name: a_name,
                    span: _,
                },
                Expr::TyNamed {
                    name: b_name,
                    span: _,
                },
            ) => {
                a_name.debug_ast_assert_eq(b_name, codebase);
            },

            _ => panic!("DebugAstEq is missing Expr branch"),
        }
    }
}
