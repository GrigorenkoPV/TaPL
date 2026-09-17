//! Inspired by rustc (although they call [`HasVisitable::visit_inner_with`] `walk` instead)

use crate::{
    ast::{
        Address, Annotation, BinOp, Binding, Catch, Decl, Expr, Function, Ident, Integer, Pattern,
        PatternBinding, Program, Type,
    },
    extensions::Extension,
};

pub(crate) trait Visitor<'src>: Sized {
    #[expect(unused_variables, reason = "default name for overrides")]
    fn visit_ident(&mut self, ident: &Ident<'src>) {}
    #[expect(unused_variables, reason = "default name for overrides")]
    fn visit_int(&mut self, int: &Integer) {}
    #[expect(unused_variables, reason = "default name for overrides")]
    fn visit_annotation(&mut self, annotation: &Annotation) {}
    #[expect(unused_variables, reason = "default name for overrides")]
    fn visit_bin_op(&mut self, binop: &BinOp) {}
    #[expect(unused_variables, reason = "default name for overrides")]
    fn visit_extension(&mut self, extension: &Extension) {}
    #[expect(unused_variables, reason = "default name for overrides")]
    fn visit_addr(&mut self, addr: &Address<'src>) {}

    fn visit_pattern_binding(&mut self, pattern_binding: &PatternBinding<'src>) {
        pattern_binding.visit_inner_with(self)
    }
    fn visit_binding(&mut self, binding: &Binding<'src>) {
        binding.visit_inner_with(self)
    }
    fn visit_function(&mut self, function: &Function<'src>) {
        function.visit_inner_with(self)
    }
    fn visit_decl(&mut self, decl: &Decl<'src>) {
        decl.visit_inner_with(self)
    }
    fn visit_ty(&mut self, ty: &Type<'src>) {
        ty.visit_inner_with(self)
    }
    fn visit_pat(&mut self, pat: &Pattern<'src>) {
        pat.visit_inner_with(self)
    }
    fn visit_catch(&mut self, catch: &Catch<'src>) {
        catch.visit_inner_with(self)
    }
    fn visit_expr(&mut self, expr: &Expr<'src>) {
        expr.visit_inner_with(self)
    }
    fn visit_program(&mut self, program: &Program<'src>) {
        program.visit_inner_with(self)
    }
}

pub(crate) trait HasVisitable<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>);
}

macro_rules! visit_opt {
    ($visitor:ident . $method:ident, $opt:ident) => {
        if let Some(inner) = $opt {
            $visitor.$method(inner)
        }
    };
}

macro_rules! visit_all {
    ($visitor:ident . $method:ident, $container:ident) => {
        for element in $container {
            $visitor.$method(element)
        }
    };
}

impl<'src> HasVisitable<'src> for Program<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        let Self { extensions, decls } = self;
        visit_all!(v.visit_extension, extensions);
        visit_all!(v.visit_decl, decls);
    }
}

impl<'src> HasVisitable<'src> for PatternBinding<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        let Self { lhs, rhs } = self;
        v.visit_pat(lhs);
        v.visit_expr(rhs);
    }
}

impl<'src> HasVisitable<'src> for Binding<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        let Self { lhs, rhs } = self;
        v.visit_ident(lhs);
        v.visit_expr(rhs);
    }
}

impl<'src> HasVisitable<'src> for Function<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        let Self {
            annotations,
            name,
            generics,
            params,
            ret_ty,
            throws,
            decls,
            ret,
        } = self;
        v.visit_ident(name);
        visit_all!(v.visit_annotation, annotations);
        visit_all!(v.visit_ident, generics);
        for (name, ty) in params {
            v.visit_ident(name);
            v.visit_ty(ty);
        }
        visit_opt!(v.visit_ty, ret_ty);
        visit_all!(v.visit_ty, throws);
        visit_all!(v.visit_decl, decls);
        v.visit_expr(ret)
    }
}

impl<'src> HasVisitable<'src> for Decl<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        match self {
            Self::Function(function) => v.visit_function(function),
            Self::TypeAlias { name, ty } | Self::ExceptionVariant { name, ty } => {
                v.visit_ident(name);
                v.visit_ty(ty);
            }
            Self::ExceptionType(ty) => v.visit_ty(ty),
        }
    }
}

impl<'src> HasVisitable<'src> for Type<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        match self {
            Self::Bool | Self::Nat | Self::Unit | Self::Top | Self::Bot | Self::Auto => {}

            Self::Ref(inner) | Self::List(inner) => v.visit_ty(inner),

            Self::Sum { left, right } => {
                v.visit_ty(left);
                v.visit_ty(right);
            }
            Self::Fn { args, ret } => {
                visit_all!(v.visit_ty, args);
                v.visit_ty(ret)
            }
            Self::Forall { binders, ty } => {
                visit_all!(v.visit_ident, binders);
                v.visit_ty(ty)
            }
            Self::Rec { var, ty } => {
                v.visit_ident(var);
                v.visit_ty(ty);
            }
            Self::Tuple(items) => visit_all!(v.visit_ty, items),
            Self::Record(items) => {
                for (name, ty) in items {
                    v.visit_ident(name);
                    v.visit_ty(ty);
                }
            }
            Self::Variant(items) => {
                for (name, ty) in items {
                    v.visit_ident(name);
                    visit_opt!(v.visit_ty, ty)
                }
            }

            Self::Var(name) => v.visit_ident(name),
        }
    }
}

impl<'src> HasVisitable<'src> for Pattern<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        match self {
            Self::False | Self::True | Self::Unit => {}

            Self::CastAs { pat, ty } | Self::Asc { pat, ty } => {
                v.visit_pat(pat);
                v.visit_ty(ty);
            }
            Self::Variant { label, pat } => {
                v.visit_ident(label);
                visit_opt!(v.visit_pat, pat)
            }
            Self::Inl(pattern) | Self::Inr(pattern) => v.visit_pat(pattern),

            Self::Tuple(patterns) | Self::List(patterns) => visit_all!(v.visit_pat, patterns),

            Self::Record(items) => {
                for (name, pat) in items {
                    v.visit_ident(name);
                    v.visit_pat(pat);
                }
            }
            Self::Cons(inner) => visit_all!(v.visit_pat, inner),
            Self::Int(int) => v.visit_int(int),
            Self::Succ(pat) => v.visit_pat(pat),
            Self::Var(name) => v.visit_ident(name),
        }
    }
}

impl<'src> HasVisitable<'src> for Catch<'src> {
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        let (pat, expr) = self;
        v.visit_pat(pat);
        v.visit_expr(expr);
    }
}

impl<'src> HasVisitable<'src> for Expr<'src> {
    #[expect(clippy::too_many_lines, reason = "giant switch")]
    fn visit_inner_with(&self, v: &mut impl Visitor<'src>) {
        match self {
            Self::True | Self::False | Self::Unit | Self::Panic => {}

            Self::New(expr)
            | Self::Deref(expr)
            | Self::Inl(expr)
            | Self::Inr(expr)
            | Self::Succ(expr)
            | Self::Not(expr)
            | Self::NatPred(expr)
            | Self::NatIsZero(expr)
            | Self::Fix(expr)
            | Self::ListHead(expr)
            | Self::ListIsEmpty(expr)
            | Self::ListTail(expr)
            | Self::Throw(expr) => v.visit_expr(expr),

            Self::Seq(exprs) | Self::Cons(exprs) => visit_all!(v.visit_expr, exprs),
            Self::NatRec(exprs) => visit_all!(v.visit_expr, exprs),
            Self::Tuple(exprs) | Self::List(exprs) => visit_all!(v.visit_expr, exprs),

            Self::Record(bindings) => visit_all!(v.visit_binding, bindings),

            Self::Let {
                rec: _,
                bindings,
                body,
            } => {
                visit_all!(v.visit_pattern_binding, bindings);
                v.visit_expr(body);
            }
            Self::Assign { lhs, rhs } => {
                v.visit_expr(lhs);
                v.visit_expr(rhs);
            }
            Self::If {
                cond,
                on_true,
                on_false,
            } => {
                v.visit_expr(cond);
                v.visit_expr(on_true);
                v.visit_expr(on_false);
            }
            Self::Var(name) => v.visit_ident(name),
            Self::Generic { generics, expr } => {
                visit_all!(v.visit_ident, generics);
                v.visit_expr(expr);
            }
            Self::BinOp { lhs, op, rhs } => {
                v.visit_expr(lhs);
                v.visit_bin_op(op);
                v.visit_expr(rhs);
            }
            Self::Fold { ty, expr }
            | Self::Unfold { ty, expr }
            | Self::Ascription { expr, ty }
            | Self::Cast { expr, ty } => {
                v.visit_ty(ty);
                v.visit_expr(expr);
            }
            Self::Fn { params, ret } => {
                for (name, ty) in params {
                    v.visit_ident(name);
                    v.visit_ty(ty);
                }
                v.visit_expr(ret);
            }
            Self::Variant { lhs, rhs } => {
                v.visit_ident(lhs);
                visit_opt!(v.visit_expr, rhs);
            }
            Self::Match { switch, cases } => {
                v.visit_expr(switch);
                for (pat, expr) in cases {
                    v.visit_pat(pat);
                    v.visit_expr(expr);
                }
            }
            Self::Application { lhs, args } => {
                v.visit_expr(lhs);
                visit_all!(v.visit_expr, args)
            }
            Self::TypeApplication { lhs, types } => {
                v.visit_expr(lhs);
                visit_all!(v.visit_ty, types)
            }
            Self::DotRecord { lhs, field } => {
                v.visit_expr(lhs);
                v.visit_ident(field);
            }
            Self::DotTuple { lhs, field: _ } => {
                v.visit_expr(lhs);
                // Theses are not really int literals
                // v.visit_int(field);
            }

            Self::TryCatch { r#try, catch } => {
                v.visit_expr(r#try);
                v.visit_catch(catch);
            }
            Self::TryWith { r#try, with } => {
                v.visit_expr(r#try);
                v.visit_expr(with);
            }
            Self::TryCastAs {
                r#try,
                r#as,
                catch,
                with,
            } => {
                v.visit_expr(r#try);
                v.visit_ty(r#as);
                v.visit_catch(catch);
                v.visit_expr(with);
            }

            Self::Integer(int) => v.visit_int(int),
            Self::Address(addr) => v.visit_addr(addr),
        }
    }
}
