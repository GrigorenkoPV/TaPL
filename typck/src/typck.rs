use core::{iter::zip, mem};
use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::HashSet,
};

use culpa::{throw, throws};

use crate::{
    ast::{Binding, Decl, Expr, Function, Ident, Integer, Pattern, PatternBinding, Program, Type},
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

fn dot_tuple(types: Vec<Type<'_>>, index: Integer) -> Option<Type<'_>> {
    let index = index.checked_sub(1)?.try_into().ok()?;
    types.into_iter().nth(index)
}

#[throws(Ident<'ctx>)]
fn check_no_duplicate_field_names<'ctx>(fields: &[Binding<'ctx>]) {
    let mut seen = HashSet::<&'ctx str>::new();
    for Binding { name, value: _ } in fields {
        if !seen.insert(name.as_ref()) {
            throw!(name.clone())
        }
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

    #[expect(clippy::too_many_lines, reason = "big sw")]
    fn infer(&self, expr: &'ctx Expr<'ctx>) -> Result<Typed<'ctx>, Error<'ctx>> {
        Ok(match expr {
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

            Expr::Fix(expr) => {
                let actual = self.infer(expr)?.into_owned();
                let Type::Fn { args, ret } = actual else {
                    throw!(Error::NotAFunction { expr, actual })
                };
                let [arg_ty] = args.as_slice() else {
                    throw!(Error::IncorrectNumberOfArguments {
                        expected: 1,
                        actual: args.len(),
                        callee: expr
                    })
                };
                match self.check_ty_coerces(&ret, arg_ty) {
                    Err(()) => throw!(Error::UnexpectedTypeForExpression {
                        expr,
                        expected: Type::Fn {
                            args: vec![ret.as_ref().to_owned()],
                            ret: ret.clone()
                        },
                        actual: Type::Fn { args, ret },
                    }),
                    Ok(()) => Owned(*ret),
                }
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

            Expr::List(elements) => match elements.as_slice() {
                [] => throw!(Error::AmbiguousList(expr)),
                [head, tail @ ..] => {
                    let element_t = self.infer(head)?;
                    for element in tail {
                        self.ensure(element, &element_t)?
                    }
                    Owned(Type::List(Box::new(element_t.into_owned())))
                }
            },

            Expr::Cons { head, tail } => {
                let element_t = self.infer(head)?;
                let list_t = Type::List(Box::new(element_t.into_owned()));
                self.ensure(tail, &list_t)?;
                Owned(list_t)
            }

            Expr::ListIsEmpty(expr) => {
                let actual = self.infer(expr)?;
                let Type::List(_) = actual.as_ref() else {
                    throw!(Error::NotAList {
                        expr,
                        actual: actual.into_owned()
                    })
                };
                Owned(Type::Bool)
            }
            Expr::ListHead(expr) => {
                let actual = self.infer(expr)?.into_owned();
                let Type::List(element_t) = actual else {
                    throw!(Error::NotAList { expr, actual })
                };
                Owned(*element_t)
            }
            Expr::ListTail(expr) => {
                let actual = self.infer(expr)?;
                let Type::List(_) = actual.as_ref() else {
                    throw!(Error::NotAList {
                        expr,
                        actual: actual.into_owned()
                    })
                };
                actual
            }

            Expr::New(expr) => {
                let inner = self.infer(expr)?;
                Owned(Type::Ref(Box::new(inner.into_owned())))
            }

            Expr::Not(expr) => {
                self.ensure(expr, &Type::Bool)?;
                Owned(Type::Bool)
            }

            &Expr::DotTuple {
                lhs: ref tuple,
                field: index,
            } => {
                // FIXME: this potentially clones too much
                let actual = self.infer(tuple)?.into_owned();
                let Type::Tuple(fields) = actual else {
                    throw!(Error::NotATuple {
                        expr: tuple,
                        actual
                    })
                };
                let actual_length = fields.len();
                Owned(
                    dot_tuple(fields, index).ok_or(Error::TupleIndexOutOfBounds {
                        tuple,
                        length: actual_length,
                        index,
                    })?,
                )
            }
            Expr::Tuple(elements) => Owned(Type::Tuple(
                elements
                    .iter()
                    .map(|element| self.infer(element).map(Cow::into_owned))
                    .collect::<Result<_, _>>()?,
            )),

            Expr::DotRecord { lhs: record, field } => {
                let actual = self.infer(record)?;
                let Type::Record(fields) = actual.as_ref() else {
                    throw!(Error::NotARecord {
                        expr: record,
                        actual: actual.into_owned()
                    })
                };
                Owned(
                    fields
                        .iter()
                        .find_map(|(name, ty)| {
                            if name == field {
                                Some(
                                    // FIXME: this clone can potentially be avoided
                                    ty.to_owned(),
                                )
                            } else {
                                None
                            }
                        })
                        .ok_or(Error::UnexpectedFieldAccess {
                            record,
                            expected: Borrowed(field),
                            actual: fields.to_owned().into(),
                        })?,
                )
            }
            Expr::Record(fields) => {
                check_no_duplicate_field_names(fields)
                    .map_err(|field| Error::DuplicateRecordFields { expr, field })?;
                Owned(Type::Record(
                    fields
                        .iter()
                        .map(|Binding { name, value }| {
                            Ok((Borrowed(name.as_ref()), self.infer(value)?.into_owned()))
                        })
                        .collect::<Result<_, _>>()?,
                ))
            }

            Expr::Assign { .. }
            | Expr::Seq(_)
            | Expr::Generic { .. }
            | Expr::BinOp { .. }
            | Expr::Cast { .. }
            | Expr::Variant { .. }
            | Expr::Match { .. }
            | Expr::Deref(_)
            | Expr::TypeApplication { .. }
            | Expr::Throw(_)
            | Expr::TryCatch { .. }
            | Expr::TryWith { .. }
            | Expr::TryCastAs { .. }
            | Expr::Address(_) => todo!("infer {expr:?}"),

            Expr::Fold { .. } | Expr::Unfold { .. } => unsupported::fold_unfold(expr),
        })
    }

    #[expect(clippy::too_many_lines, reason = "giant match")]
    fn ensure(&self, expr: &'ctx Expr<'ctx>, expected: &Type<'ctx>) -> Result<(), Error<'ctx>> {
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

            // If
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

            // Lambda
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

            // NatRec
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

            // Fix
            (Expr::Fix(expr), ty) => self
                .ensure(
                    expr,
                    &Type::Fn {
                        ret: Box::new(ty.clone()),
                        args: vec![ty.to_owned()],
                    },
                )
                .map_err(|e| {
                    if let Ok(actual) = self.infer(expr)
                        && !matches!(actual.as_ref(), Type::Fn { .. })
                    {
                        Error::NotAFunction {
                            expr,
                            actual: actual.into_owned(),
                        }
                    } else {
                        e
                    }
                })?,

            // Sum
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
                injection: expr,
                expected: expected.to_owned()
            }),

            // Lists
            (Expr::List(elements), Type::List(element_t)) => {
                for element in elements {
                    self.ensure(element, element_t)?
                }
            }
            (Expr::Cons { head, tail }, list_t @ Type::List(element_t)) => {
                self.ensure(head, element_t)?;
                self.ensure(tail, list_t)?
            }

            (Expr::ListTail(list), list_t @ Type::List(_)) => self.ensure(list, list_t)?,

            (
                Expr::List(_) | Expr::Cons { .. } | Expr::ListTail(_),
                Type::Bool
                | Type::Nat
                | Type::Ref(_)
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::Variant(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedList {
                list: expr,
                expected: expected.to_owned()
            }),

            (Expr::ListHead(list), element_t) => {
                self.ensure(
                    list,
                    &Type::List(
                        // FIXME: `Rc` / `Cow`
                        Box::new(element_t.to_owned()),
                    ),
                )?
            }

            // Tuples
            (Expr::Tuple(actual), Type::Tuple(expected)) => {
                {
                    let expected = expected.len();
                    let actual = actual.len();
                    if actual != expected {
                        throw!(Error::UnexpectedTupleLength {
                            tuple: expr,
                            actual,
                            expected
                        })
                    }
                }
                for (actual, expected) in zip(actual, expected) {
                    self.ensure(actual, expected)?
                }
            }
            (
                Expr::Tuple(_),
                Type::Bool
                | Type::Nat
                | Type::Ref(_)
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Record(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedTuple {
                tuple: expr,
                expected: expected.to_owned()
            }),

            (Expr::Record(_), Type::Record(_)) => todo!("typck {expr:?}"),
            (
                Expr::Record { .. },
                Type::Bool
                | Type::Nat
                | Type::Ref(_)
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedRecord {
                record: expr,
                expected: expected.to_owned()
            }),

            // fallback to infer + check_ty_coerces
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
                | Expr::Application { .. }
                | Expr::DotRecord { .. }
                | Expr::DotTuple { .. },
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

            // TODO:
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
        Ok(())
    }

    #[expect(clippy::self_only_used_in_recursion, reason = "WIP")]
    // TODO: is this just equality?
    fn check_ty_coerces(&self, from: &Type<'_>, to: &Type<'_>) -> Result<(), ()> {
        match [from, to] {
            [rec @ Type::Rec { .. }, _] | [_, rec @ Type::Rec { .. }] => unsupported::mu(rec),

            [Type::Bool, Type::Bool] | [Type::Nat, Type::Nat] | [Type::Unit, Type::Unit] => Ok(()),
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

            [Type::List(from), Type::List(to)] => self.check_ty_coerces(from, to),

            [from, to] => {
                assert_ne!(
                    mem::discriminant(from),
                    mem::discriminant(to),
                    "TODO: check coercion from {from:?} to {to:?}"
                );
                Err(())
            }
        }
    }
}

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    gates::check_extensions(program)?;
    check_main(program)?;

    Context::check_program(program)?;
}
