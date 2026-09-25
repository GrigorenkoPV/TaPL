use core::iter::zip;
use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::HashSet,
};

use culpa::{throw, throws};

use crate::{
    ast::{Decl, Expr, Function, Ident, Pattern, PatternBinding, Program, Type},
    extensions::unsupported,
};

mod error;
mod gates;

pub(crate) use self::error::Error;

#[throws(Error<'a>)]
fn check_main<'a>(program: &'a Program<'a>) {
    let mains: Vec<&Function<'_>> = program
        .decls
        .iter()
        .filter_map(|decl| {
            if let Decl::Function(function) = decl
                && function.name == "main"
            {
                Some(function.as_ref())
            } else {
                None
            }
        })
        .collect();
    match mains.as_slice() {
        [] => throw!(Error::MissingMain),
        [one, _two, ..] => {
            throw!(Error::DuplicateFunctionDeclaration(Borrowed(&one.name)))
        }
        [main] => {
            let arity = main.params.len();
            if arity != 1 {
                throw!(Error::IncorrectArityOfMain(arity))
            }
        }
    }
}

type Typed<'ctx> = Cow<'ctx, Type<'ctx>>;
type Namespace<'a, 'ctx> = crate::namespace::Namespace<'a, Ident<'ctx>, Typed<'ctx>>;

struct Context<'a, 'ctx> {
    type_aliases: Namespace<'a, 'ctx>,
    variables: Namespace<'a, 'ctx>,
}

fn nat_rec_step_type(acc: Type<'_>) -> Type<'_> {
    Type::Fn {
        args: vec![Type::Nat],
        ret: Box::new(Type::Fn {
            // TODO: `Cow`/`Rc`?
            args: vec![acc.clone()],
            ret: Box::new(acc),
        }),
    }
}

impl<'ctx> Context<'_, 'ctx> {
    #[throws(Error<'ctx>)]
    fn check_program(program: &'ctx Program<'ctx>) {
        Self {
            type_aliases: Namespace::empty(),
            variables: Namespace::empty(),
        }
        .check_decls(program.decls.iter())?
    }

    #[must_use]
    fn new_scope<'this>(&'this self) -> Context<'this, 'ctx> {
        let Self {
            type_aliases,
            variables,
        } = self;
        Context {
            type_aliases: type_aliases.new_scope(),
            variables: variables.new_scope(),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_decls(&mut self, decls: impl IntoIterator<Item = &'ctx Decl<'ctx>>) {
        for decl in decls {
            self.check_decl(decl)?
        }
    }

    #[throws(Error<'ctx>)]
    fn check_decl(&mut self, decl: &'ctx Decl<'ctx>) {
        match decl {
            Decl::Function(function) => self.check_function(function)?,
            Decl::TypeAlias { name, ty } => {
                self.check_ty_wf(ty)?;
                self.type_aliases
                    .add_local(Borrowed(name), Borrowed(ty))
                    .map_err(|(new, old)| {
                        todo!("Duplicate type alias {name:?}: old: {old:?}, new: {new:?}")
                    })?
            }
            Decl::ExceptionType(_) | Decl::ExceptionVariant { .. } => todo!(),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_ty_wf(&self, ty: &'ctx Type<'ctx>) {
        match ty {
            Type::Bool | Type::Nat | Type::Unit | Type::Top | Type::Bot | Type::Auto => {}

            Type::Ref(inner) | Type::List(inner) => self.check_ty_wf(inner)?,

            Type::Sum { left, right } => {
                self.check_ty_wf(left)?;
                self.check_ty_wf(right)?;
            }
            Type::Tuple(inner) => {
                for t in inner {
                    self.check_ty_wf(t)?
                }
            }
            Type::Forall { binders, ty } => {
                let mut ctx = self.new_scope();
                for name in binders {
                    ctx.type_aliases
                        .add_local(Borrowed(name), Owned(Type::Auto))
                        .map_err(|(_old, _new)| Error::DuplicateTypeParameter(Borrowed(name)))?;
                }
                ctx.check_ty_wf(ty)?
            }

            Type::Fn { args, ret } => {
                for arg in args {
                    self.check_ty_wf(arg)?
                }
                self.check_ty_wf(ret)?
            }

            Type::Record(fields) => {
                let mut names = HashSet::<&str>::new();
                for (name, ty) in fields {
                    if !names.insert(name) {
                        throw!(Error::DuplicateRecordTypeFields(Borrowed(name)))
                    }
                    self.check_ty_wf(ty)?
                }
            }
            Type::Variant(variants) => {
                let mut names = HashSet::<&str>::new();
                for (name, ty) in variants {
                    if !names.insert(name) {
                        throw!(Error::DuplicateVariantTypeFields(Borrowed(name)))
                    }
                    if let Some(ty) = ty {
                        self.check_ty_wf(ty)?
                    }
                }
            }

            Type::Var(name) => match self.type_aliases.lookup(name) {
                Some(_) => {}
                None => throw!(Error::UndefinedTypeVariable(Borrowed(name))),
            },

            t @ Type::Rec { .. } => unsupported::mu(t),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_function(&mut self, function: &'ctx Function<'ctx>) {
        let Function {
            annotations,
            name: fn_name,
            generics,
            params,
            ret_ty,
            throws,
            decls,
            body,
        } = function;
        assert!(
            annotations.is_empty(),
            "Functions with annotations are not supported"
        );
        if !generics.is_empty() {
            todo!()
        }
        assert!(throws.is_empty(), "Functions that throw are not supported");
        let ret_ty = ret_ty
            .as_ref()
            .expect("Function without a return type are not supported");

        self.check_ty_wf(ret_ty)?;
        self.variables
            .add_local(
                Borrowed(fn_name),
                Owned(Type::Fn {
                    args: params
                        .iter()
                        .map(
                            |(_name, ty)| ty.clone(), // FIXME: use `Cow`?
                        )
                        .collect(),
                    ret: Box::new(ret_ty.clone()), // FIXME: use `Cow`?
                }),
            )
            .map_err(|(_old, _new)| Error::DuplicateFunctionDeclaration(Borrowed(fn_name)))?;

        let mut ctx = self.new_scope();
        for (name, ty) in params {
            self.check_ty_wf(ty)?;
            ctx.variables
                .add_local(Borrowed(name), Borrowed(ty))
                .map_err(|(_old, _new)| Error::DuplicateFunctionParameter {
                    function: Borrowed(fn_name),
                    param: Borrowed(name),
                })?
        }
        ctx.check_decls(decls)?;
        ctx.ensure(body, ret_ty)?
    }

    #[throws(Error<'ctx>)]
    fn check_bindings(&mut self, bindings: impl IntoIterator<Item = &'ctx PatternBinding<'ctx>>) {
        for binding in bindings {
            self.check_binding(binding)?
        }
    }

    #[throws(Error<'ctx>)]
    fn check_binding(&mut self, binding: &'ctx PatternBinding<'ctx>) {
        let PatternBinding { lhs, rhs } = binding;
        match lhs {
            Pattern::Unit => self.ensure(rhs, &Type::Unit)?,
            Pattern::Var(var) => {
                let ty = self.infer(rhs)?;
                self.variables
                    .add_local(Borrowed(var), ty)
                    .map_err(|(_old, _new)| Error::DuplicateLetBinding(Borrowed(var)))?
            }
            Pattern::CastAs { .. }
            | Pattern::Asc { .. }
            | Pattern::Variant { .. }
            | Pattern::Inl(_)
            | Pattern::Inr(_)
            | Pattern::Tuple(_)
            | Pattern::Record(_)
            | Pattern::List(_)
            | Pattern::Cons(_)
            | Pattern::False
            | Pattern::True
            | Pattern::Int(_)
            | Pattern::Succ(_) => todo!("let binding {binding:?}"),
        }
    }

    #[throws(Error<'ctx>)]
    #[expect(clippy::too_many_lines, reason = "big sw")]
    fn infer(&self, expr: &'ctx Expr<'ctx>) -> Typed<'ctx> {
        match expr {
            Expr::True | Expr::False => Owned(Type::Bool),
            Expr::Unit => Owned(Type::Unit),
            Expr::Integer(_) => Owned(Type::Nat),
            Expr::Panic => Owned(Type::Auto),

            Expr::Var(var) => self
                .variables
                .lookup(var)
                .ok_or(Error::UndefinedVariable(Borrowed(var)))?
                .clone(),

            Expr::NatIsZero(nat) => {
                self.ensure(nat, &Type::Nat)?;
                Owned(Type::Bool)
            }

            Expr::Succ(nat) | Expr::NatPred(nat) => {
                self.ensure(nat, &Type::Nat)?;
                Owned(Type::Nat)
            }

            Expr::If {
                cond,
                on_true,
                on_false,
            } => {
                self.ensure(cond, &Type::Bool)?;
                let ty = self.infer(on_true)?;
                self.ensure(on_false, &ty)?;
                ty
            }

            Expr::Let {
                rec,
                bindings,
                body,
            } => {
                if *rec {
                    todo!("infer letrec")
                }
                let mut ctx = self.new_scope();
                ctx.check_bindings(bindings)?;
                ctx.infer(body)?
            }

            Expr::NatRec {
                iterations,
                start,
                step,
            } => {
                self.ensure(iterations, &Type::Nat)?;
                let res = self.infer(start)?;
                self.ensure(step, &nat_rec_step_type(res.clone().into_owned()))?;
                res
            }

            Expr::Application {
                lhs,
                args: args_actual,
            } => {
                let callee = self.infer(lhs)?;
                let Type::Fn {
                    args: args_expected,
                    ret,
                } = callee.as_ref()
                else {
                    throw!(Error::NotAFunction {
                        expr: lhs,
                        actual: callee.into_owned()
                    })
                };
                {
                    let actual = args_actual.len();
                    let expected = args_expected.len();
                    if actual != expected {
                        throw!(Error::IncorrectNumberOfArguments {
                            expected,
                            actual,
                            callee: lhs
                        })
                    }
                }
                for (actual, expected) in zip(args_actual, args_expected) {
                    self.ensure(actual, expected)?
                }
                Owned(ret.as_ref().to_owned())
            }

            Expr::Abstraction { params, ret } => {
                let mut ctx = self.new_scope();
                let args = params
                    .iter()
                    .map(|(name, ty)| {
                        ctx.variables
                            .add_local(Borrowed(name), Borrowed(ty))
                            .map_err(|(_old, _new)| Error::DuplicateFunctionParameter {
                                function: Owned(expr.to_string()),
                                param: Borrowed(name),
                            })
                            .map(|()| ty.to_owned())
                    })
                    .collect::<Result<_, _>>()?;
                let ret = ctx.infer(ret)?;
                Owned(Type::Fn {
                    args,
                    ret: Box::new(ret.into_owned()),
                })
            }

            Expr::Ascription { expr, ty } => {
                self.ensure(expr, ty)?;
                Borrowed(ty)
            }

            Expr::Inl(_) | Expr::Inr(_) => throw!(Error::AmbiguousSumType(expr)),

            Expr::Assign { .. }
            | Expr::Seq(_)
            | Expr::Generic { .. }
            | Expr::BinOp { .. }
            | Expr::Cast { .. }
            | Expr::Variant { .. }
            | Expr::Match { .. }
            | Expr::List(_)
            | Expr::New(_)
            | Expr::Deref(_)
            | Expr::TypeApplication { .. }
            | Expr::DotRecord { .. }
            | Expr::DotTuple { .. }
            | Expr::Tuple(_)
            | Expr::Cons(_)
            | Expr::Record(_)
            | Expr::ListHead(_)
            | Expr::ListIsEmpty(_)
            | Expr::ListTail(_)
            | Expr::Throw(_)
            | Expr::TryCatch { .. }
            | Expr::TryWith { .. }
            | Expr::TryCastAs { .. }
            | Expr::Not(_)
            | Expr::Fix(_)
            | Expr::Address(_) => todo!("infer {expr:?}"),

            Expr::Fold { .. } | Expr::Unfold { .. } => unsupported::fold_unfold(expr),
        }
    }

    #[expect(clippy::too_many_lines, reason = "giant match")]
    #[throws(Error<'ctx>)]
    fn ensure(&self, expr: &'ctx Expr<'ctx>, expected: &Type<'ctx>) {
        match (expr, expected) {
            // Unsupported
            (e @ (Expr::Fold { .. } | Expr::Unfold { .. }), _) => unsupported::fold_unfold(e),
            (_, t @ Type::Rec { .. }) => unsupported::mu(t),

            // Special
            (_, Type::Top) => todo!("top"),
            (_, Type::Bot) => todo!("bot"),
            (_, Type::Auto) => todo!("auto"),
            (_, Type::Forall { .. }) => todo!("forall"),

            // Var
            (expr, Type::Var(var)) => {
                let expected = self
                    .type_aliases
                    .lookup(var)
                    .ok_or(Error::UndefinedTypeVariable(var.clone()))?;
                self.ensure(expr, expected)?
            }

            // Let
            (
                Expr::Let {
                    rec,
                    bindings,
                    body,
                },
                ty,
            ) => {
                if *rec {
                    todo!("typck letrec")
                }
                let mut ctx = self.new_scope();
                ctx.check_bindings(bindings)?;
                ctx.ensure(body, ty)?
            }

            (
                Expr::If {
                    cond,
                    on_true,
                    on_false,
                },
                ty,
            ) => {
                self.ensure(cond, &Type::Bool)?;
                self.ensure(on_true, ty)?;
                self.ensure(on_false, ty)?;
            }

            (
                Expr::Abstraction {
                    params: args_actual,
                    ret: ret_actual,
                },
                Type::Fn {
                    args: args_expected,
                    ret: ret_expected,
                },
            ) => {
                {
                    let expected = args_expected.len();
                    let actual = args_actual.len();
                    if expected != actual {
                        throw!(Error::UnexpectedNumberOfParametersInLambda {
                            expected,
                            actual,
                            lambda: expr
                        })
                    }
                }
                let mut ctx = self.new_scope();
                for ((name, actual), expected) in zip(args_actual, args_expected) {
                    self.check_ty_coerces(expected, actual).map_err(|()| {
                        Error::UnexpectedTypeForParameter {
                            param: Borrowed(name),
                            actual: actual.to_owned(),
                            expected: expected.to_owned(),
                        }
                    })?;
                    ctx.variables
                        .add_local(Borrowed(name), Borrowed(actual))
                        .map_err(|(_old, _new)| Error::DuplicateFunctionParameter {
                            function: Owned(expr.to_string()),
                            param: Borrowed(name),
                        })?
                }
                ctx.ensure(ret_actual, ret_expected)?
            }
            (
                Expr::Abstraction { .. },
                Type::Bool
                | Type::Nat { .. }
                | Type::Ref { .. }
                | Type::Sum { .. }
                | Type::Tuple { .. }
                | Type::Record { .. }
                | Type::Variant { .. }
                | Type::List { .. }
                | Type::Unit { .. },
            ) => throw!(Error::UnexpectedLambda {
                lambda: expr,
                expected: expected.to_owned(),
            }),

            (
                Expr::NatRec {
                    iterations,
                    start,
                    step,
                },
                expected,
            ) => {
                self.ensure(iterations, &Type::Nat)?;
                self.ensure(start, expected)?;
                self.ensure(step, &nat_rec_step_type(expected.to_owned()))?;
            }

            (
                Expr::Inl(expr),
                Type::Sum {
                    left: expected,
                    right: _other,
                },
            )
            | (
                Expr::Inr(expr),
                Type::Sum {
                    left: _other,
                    right: expected,
                },
            ) => {
                self.ensure(expr, expected)?;
                // TODO:
                // self.check_ty_wf(other)?
            }

            (
                Expr::Inl(_) | Expr::Inr(_),
                Type::Bool
                | Type::Nat
                | Type::Ref(_)
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedInjection {
                expr,
                expected: expected.to_owned()
            }),

            (
                Expr::Var(_)
                | Expr::Unit
                | Expr::True
                | Expr::False
                | Expr::Integer(_)
                | Expr::Panic
                | Expr::Not(_)
                | Expr::NatIsZero(_)
                | Expr::ListIsEmpty(_)
                | Expr::Succ(_)
                | Expr::NatPred(_)
                | Expr::Ascription { .. }
                | Expr::Application { .. },
                Type::Bool
                | Type::Nat
                | Type::Ref(_)
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
            ) => {
                let actual = self.infer(expr)?;
                self.check_ty_coerces(&actual, expected).map_err(|()| {
                    Error::UnexpectedTypeForExpression {
                        expr,
                        actual: actual.clone().into_owned(),
                        expected: expected.to_owned(),
                    }
                })?;
            }

            #[expect(clippy::unnested_or_patterns, reason = "WIP")]
            (Expr::Seq { .. }, Type::Bool)
            | (Expr::Seq { .. }, Type::Nat { .. })
            | (Expr::Seq { .. }, Type::Ref { .. })
            | (Expr::Seq { .. }, Type::Sum { .. })
            | (Expr::Seq { .. }, Type::Fn { .. })
            | (Expr::Seq { .. }, Type::Tuple { .. })
            | (Expr::Seq { .. }, Type::Record { .. })
            | (Expr::Seq { .. }, Type::Variant { .. })
            | (Expr::Seq { .. }, Type::List { .. })
            | (Expr::Seq { .. }, Type::Unit { .. })
            | (Expr::Assign { .. }, Type::Bool)
            | (Expr::Assign { .. }, Type::Nat { .. })
            | (Expr::Assign { .. }, Type::Ref { .. })
            | (Expr::Assign { .. }, Type::Sum { .. })
            | (Expr::Assign { .. }, Type::Fn { .. })
            | (Expr::Assign { .. }, Type::Tuple { .. })
            | (Expr::Assign { .. }, Type::Record { .. })
            | (Expr::Assign { .. }, Type::Variant { .. })
            | (Expr::Assign { .. }, Type::List { .. })
            | (Expr::Assign { .. }, Type::Unit { .. })
            | (Expr::Generic { .. }, Type::Bool)
            | (Expr::Generic { .. }, Type::Nat { .. })
            | (Expr::Generic { .. }, Type::Ref { .. })
            | (Expr::Generic { .. }, Type::Sum { .. })
            | (Expr::Generic { .. }, Type::Fn { .. })
            | (Expr::Generic { .. }, Type::Tuple { .. })
            | (Expr::Generic { .. }, Type::Record { .. })
            | (Expr::Generic { .. }, Type::Variant { .. })
            | (Expr::Generic { .. }, Type::List { .. })
            | (Expr::Generic { .. }, Type::Unit { .. })
            | (Expr::BinOp { .. }, Type::Bool)
            | (Expr::BinOp { .. }, Type::Nat { .. })
            | (Expr::BinOp { .. }, Type::Ref { .. })
            | (Expr::BinOp { .. }, Type::Sum { .. })
            | (Expr::BinOp { .. }, Type::Fn { .. })
            | (Expr::BinOp { .. }, Type::Tuple { .. })
            | (Expr::BinOp { .. }, Type::Record { .. })
            | (Expr::BinOp { .. }, Type::Variant { .. })
            | (Expr::BinOp { .. }, Type::List { .. })
            | (Expr::BinOp { .. }, Type::Unit { .. })
            | (Expr::Cast { .. }, Type::Bool)
            | (Expr::Cast { .. }, Type::Nat { .. })
            | (Expr::Cast { .. }, Type::Ref { .. })
            | (Expr::Cast { .. }, Type::Sum { .. })
            | (Expr::Cast { .. }, Type::Fn { .. })
            | (Expr::Cast { .. }, Type::Tuple { .. })
            | (Expr::Cast { .. }, Type::Record { .. })
            | (Expr::Cast { .. }, Type::Variant { .. })
            | (Expr::Cast { .. }, Type::List { .. })
            | (Expr::Cast { .. }, Type::Unit { .. })
            | (Expr::Variant { .. }, Type::Bool)
            | (Expr::Variant { .. }, Type::Nat { .. })
            | (Expr::Variant { .. }, Type::Ref { .. })
            | (Expr::Variant { .. }, Type::Sum { .. })
            | (Expr::Variant { .. }, Type::Fn { .. })
            | (Expr::Variant { .. }, Type::Tuple { .. })
            | (Expr::Variant { .. }, Type::Record { .. })
            | (Expr::Variant { .. }, Type::Variant { .. })
            | (Expr::Variant { .. }, Type::List { .. })
            | (Expr::Variant { .. }, Type::Unit { .. })
            | (Expr::Match { .. }, Type::Bool)
            | (Expr::Match { .. }, Type::Nat { .. })
            | (Expr::Match { .. }, Type::Ref { .. })
            | (Expr::Match { .. }, Type::Sum { .. })
            | (Expr::Match { .. }, Type::Fn { .. })
            | (Expr::Match { .. }, Type::Tuple { .. })
            | (Expr::Match { .. }, Type::Record { .. })
            | (Expr::Match { .. }, Type::Variant { .. })
            | (Expr::Match { .. }, Type::List { .. })
            | (Expr::Match { .. }, Type::Unit { .. })
            | (Expr::List { .. }, Type::Bool)
            | (Expr::List { .. }, Type::Nat { .. })
            | (Expr::List { .. }, Type::Ref { .. })
            | (Expr::List { .. }, Type::Sum { .. })
            | (Expr::List { .. }, Type::Fn { .. })
            | (Expr::List { .. }, Type::Tuple { .. })
            | (Expr::List { .. }, Type::Record { .. })
            | (Expr::List { .. }, Type::Variant { .. })
            | (Expr::List { .. }, Type::List { .. })
            | (Expr::List { .. }, Type::Unit { .. })
            | (Expr::New { .. }, Type::Bool)
            | (Expr::New { .. }, Type::Nat { .. })
            | (Expr::New { .. }, Type::Ref { .. })
            | (Expr::New { .. }, Type::Sum { .. })
            | (Expr::New { .. }, Type::Fn { .. })
            | (Expr::New { .. }, Type::Tuple { .. })
            | (Expr::New { .. }, Type::Record { .. })
            | (Expr::New { .. }, Type::Variant { .. })
            | (Expr::New { .. }, Type::List { .. })
            | (Expr::New { .. }, Type::Unit { .. })
            | (Expr::Deref { .. }, Type::Bool)
            | (Expr::Deref { .. }, Type::Nat { .. })
            | (Expr::Deref { .. }, Type::Ref { .. })
            | (Expr::Deref { .. }, Type::Sum { .. })
            | (Expr::Deref { .. }, Type::Fn { .. })
            | (Expr::Deref { .. }, Type::Tuple { .. })
            | (Expr::Deref { .. }, Type::Record { .. })
            | (Expr::Deref { .. }, Type::Variant { .. })
            | (Expr::Deref { .. }, Type::List { .. })
            | (Expr::Deref { .. }, Type::Unit { .. })
            | (Expr::TypeApplication { .. }, Type::Bool)
            | (Expr::TypeApplication { .. }, Type::Nat { .. })
            | (Expr::TypeApplication { .. }, Type::Ref { .. })
            | (Expr::TypeApplication { .. }, Type::Sum { .. })
            | (Expr::TypeApplication { .. }, Type::Fn { .. })
            | (Expr::TypeApplication { .. }, Type::Tuple { .. })
            | (Expr::TypeApplication { .. }, Type::Record { .. })
            | (Expr::TypeApplication { .. }, Type::Variant { .. })
            | (Expr::TypeApplication { .. }, Type::List { .. })
            | (Expr::TypeApplication { .. }, Type::Unit { .. })
            | (Expr::DotRecord { .. }, Type::Bool)
            | (Expr::DotRecord { .. }, Type::Nat { .. })
            | (Expr::DotRecord { .. }, Type::Ref { .. })
            | (Expr::DotRecord { .. }, Type::Sum { .. })
            | (Expr::DotRecord { .. }, Type::Fn { .. })
            | (Expr::DotRecord { .. }, Type::Tuple { .. })
            | (Expr::DotRecord { .. }, Type::Record { .. })
            | (Expr::DotRecord { .. }, Type::Variant { .. })
            | (Expr::DotRecord { .. }, Type::List { .. })
            | (Expr::DotRecord { .. }, Type::Unit { .. })
            | (Expr::DotTuple { .. }, Type::Bool)
            | (Expr::DotTuple { .. }, Type::Nat { .. })
            | (Expr::DotTuple { .. }, Type::Ref { .. })
            | (Expr::DotTuple { .. }, Type::Sum { .. })
            | (Expr::DotTuple { .. }, Type::Fn { .. })
            | (Expr::DotTuple { .. }, Type::Tuple { .. })
            | (Expr::DotTuple { .. }, Type::Record { .. })
            | (Expr::DotTuple { .. }, Type::Variant { .. })
            | (Expr::DotTuple { .. }, Type::List { .. })
            | (Expr::DotTuple { .. }, Type::Unit { .. })
            | (Expr::Tuple { .. }, Type::Bool)
            | (Expr::Tuple { .. }, Type::Nat { .. })
            | (Expr::Tuple { .. }, Type::Ref { .. })
            | (Expr::Tuple { .. }, Type::Sum { .. })
            | (Expr::Tuple { .. }, Type::Fn { .. })
            | (Expr::Tuple { .. }, Type::Tuple { .. })
            | (Expr::Tuple { .. }, Type::Record { .. })
            | (Expr::Tuple { .. }, Type::Variant { .. })
            | (Expr::Tuple { .. }, Type::List { .. })
            | (Expr::Tuple { .. }, Type::Unit { .. })
            | (Expr::Cons { .. }, Type::Bool)
            | (Expr::Cons { .. }, Type::Nat { .. })
            | (Expr::Cons { .. }, Type::Ref { .. })
            | (Expr::Cons { .. }, Type::Sum { .. })
            | (Expr::Cons { .. }, Type::Fn { .. })
            | (Expr::Cons { .. }, Type::Tuple { .. })
            | (Expr::Cons { .. }, Type::Record { .. })
            | (Expr::Cons { .. }, Type::Variant { .. })
            | (Expr::Cons { .. }, Type::List { .. })
            | (Expr::Cons { .. }, Type::Unit { .. })
            | (Expr::Record { .. }, Type::Bool)
            | (Expr::Record { .. }, Type::Nat { .. })
            | (Expr::Record { .. }, Type::Ref { .. })
            | (Expr::Record { .. }, Type::Sum { .. })
            | (Expr::Record { .. }, Type::Fn { .. })
            | (Expr::Record { .. }, Type::Tuple { .. })
            | (Expr::Record { .. }, Type::Record { .. })
            | (Expr::Record { .. }, Type::Variant { .. })
            | (Expr::Record { .. }, Type::List { .. })
            | (Expr::Record { .. }, Type::Unit { .. })
            | (Expr::ListHead { .. }, Type::Bool)
            | (Expr::ListHead { .. }, Type::Nat { .. })
            | (Expr::ListHead { .. }, Type::Ref { .. })
            | (Expr::ListHead { .. }, Type::Sum { .. })
            | (Expr::ListHead { .. }, Type::Fn { .. })
            | (Expr::ListHead { .. }, Type::Tuple { .. })
            | (Expr::ListHead { .. }, Type::Record { .. })
            | (Expr::ListHead { .. }, Type::Variant { .. })
            | (Expr::ListHead { .. }, Type::List { .. })
            | (Expr::ListHead { .. }, Type::Unit { .. })
            | (Expr::ListTail { .. }, Type::Bool)
            | (Expr::ListTail { .. }, Type::Nat { .. })
            | (Expr::ListTail { .. }, Type::Ref { .. })
            | (Expr::ListTail { .. }, Type::Sum { .. })
            | (Expr::ListTail { .. }, Type::Fn { .. })
            | (Expr::ListTail { .. }, Type::Tuple { .. })
            | (Expr::ListTail { .. }, Type::Record { .. })
            | (Expr::ListTail { .. }, Type::Variant { .. })
            | (Expr::ListTail { .. }, Type::List { .. })
            | (Expr::ListTail { .. }, Type::Unit { .. })
            | (Expr::Throw { .. }, Type::Bool)
            | (Expr::Throw { .. }, Type::Nat { .. })
            | (Expr::Throw { .. }, Type::Ref { .. })
            | (Expr::Throw { .. }, Type::Sum { .. })
            | (Expr::Throw { .. }, Type::Fn { .. })
            | (Expr::Throw { .. }, Type::Tuple { .. })
            | (Expr::Throw { .. }, Type::Record { .. })
            | (Expr::Throw { .. }, Type::Variant { .. })
            | (Expr::Throw { .. }, Type::List { .. })
            | (Expr::Throw { .. }, Type::Unit { .. })
            | (Expr::TryCatch { .. }, Type::Bool)
            | (Expr::TryCatch { .. }, Type::Nat { .. })
            | (Expr::TryCatch { .. }, Type::Ref { .. })
            | (Expr::TryCatch { .. }, Type::Sum { .. })
            | (Expr::TryCatch { .. }, Type::Fn { .. })
            | (Expr::TryCatch { .. }, Type::Tuple { .. })
            | (Expr::TryCatch { .. }, Type::Record { .. })
            | (Expr::TryCatch { .. }, Type::Variant { .. })
            | (Expr::TryCatch { .. }, Type::List { .. })
            | (Expr::TryCatch { .. }, Type::Unit { .. })
            | (Expr::TryWith { .. }, Type::Bool)
            | (Expr::TryWith { .. }, Type::Nat { .. })
            | (Expr::TryWith { .. }, Type::Ref { .. })
            | (Expr::TryWith { .. }, Type::Sum { .. })
            | (Expr::TryWith { .. }, Type::Fn { .. })
            | (Expr::TryWith { .. }, Type::Tuple { .. })
            | (Expr::TryWith { .. }, Type::Record { .. })
            | (Expr::TryWith { .. }, Type::Variant { .. })
            | (Expr::TryWith { .. }, Type::List { .. })
            | (Expr::TryWith { .. }, Type::Unit { .. })
            | (Expr::TryCastAs { .. }, Type::Bool)
            | (Expr::TryCastAs { .. }, Type::Nat { .. })
            | (Expr::TryCastAs { .. }, Type::Ref { .. })
            | (Expr::TryCastAs { .. }, Type::Sum { .. })
            | (Expr::TryCastAs { .. }, Type::Fn { .. })
            | (Expr::TryCastAs { .. }, Type::Tuple { .. })
            | (Expr::TryCastAs { .. }, Type::Record { .. })
            | (Expr::TryCastAs { .. }, Type::Variant { .. })
            | (Expr::TryCastAs { .. }, Type::List { .. })
            | (Expr::TryCastAs { .. }, Type::Unit { .. })
            | (Expr::Fix { .. }, Type::Bool)
            | (Expr::Fix { .. }, Type::Nat { .. })
            | (Expr::Fix { .. }, Type::Ref { .. })
            | (Expr::Fix { .. }, Type::Sum { .. })
            | (Expr::Fix { .. }, Type::Fn { .. })
            | (Expr::Fix { .. }, Type::Tuple { .. })
            | (Expr::Fix { .. }, Type::Record { .. })
            | (Expr::Fix { .. }, Type::Variant { .. })
            | (Expr::Fix { .. }, Type::List { .. })
            | (Expr::Fix { .. }, Type::Unit { .. })
            | (Expr::Address { .. }, Type::Bool)
            | (Expr::Address { .. }, Type::Nat { .. })
            | (Expr::Address { .. }, Type::Ref { .. })
            | (Expr::Address { .. }, Type::Sum { .. })
            | (Expr::Address { .. }, Type::Fn { .. })
            | (Expr::Address { .. }, Type::Tuple { .. })
            | (Expr::Address { .. }, Type::Record { .. })
            | (Expr::Address { .. }, Type::Variant { .. })
            | (Expr::Address { .. }, Type::List { .. })
            | (Expr::Address { .. }, Type::Unit { .. }) => todo!("typck {expr:?} is {expected:?}"),
        }
    }

    #[expect(clippy::self_only_used_in_recursion, reason = "WIP")]
    // TODO: is this just equality?
    fn check_ty_coerces(&self, from: &Type<'_>, to: &Type<'_>) -> Result<(), ()> {
        match [from, to] {
            [Type::Bool, Type::Bool] | [Type::Nat, Type::Nat] | [Type::Unit, Type::Unit] => Ok(()),
            [Type::Nat, Type::Bool] | [Type::Bool, Type::Nat] => Err(()),
            [
                Type::Fn {
                    args: args_from,
                    ret: ret_from,
                },
                Type::Fn {
                    args: args_to,
                    ret: ret_to,
                },
            ] => {
                if args_from.len() != args_to.len() {
                    throw!(())
                }
                for (arg_from, arg_to) in zip(args_from, args_to) {
                    self.check_ty_coerces(arg_to, arg_from)?
                }
                self.check_ty_coerces(ret_from, ret_to)
            }
            [
                Type::Sum {
                    left: from_left,
                    right: from_right,
                },
                Type::Sum {
                    left: to_left,
                    right: to_right,
                },
            ] => {
                self.check_ty_coerces(from_left, to_left)?;
                self.check_ty_coerces(from_right, to_right)?;
                Ok(())
            }
            [from, to] => todo!("Check coercion from {from:?} to {to:?}"),
        }
    }
}

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    gates::check_extensions(program)?;
    check_main(program)?;

    Context::check_program(program)?;
}
