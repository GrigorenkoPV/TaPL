use core::iter::zip;
use std::borrow::Cow::{Borrowed, Owned};

use culpa::throw;

use crate::{
    ast::{Binding, Expr, Type},
    extensions::unsupported,
};

use super::{Context, Error, check_no_duplicate_field_names, nat_rec_step_type};

impl<'ctx> Context<'_, 'ctx> {
    #[expect(clippy::too_many_lines, reason = "giant match")]
    pub(super) fn ensure(
        &self,
        expr: &'ctx Expr<'ctx>,
        expected: &Type<'ctx>,
    ) -> Result<(), Error<'ctx>> {
        match (expr, expected) {
            // Unsupported
            (e @ (Expr::Fold { .. } | Expr::Unfold { .. }), _) => unsupported::fold_unfold(e),
            (_, t @ Type::Rec { .. }) => unsupported::mu(t),

            // Special types
            (_, Type::Auto) => todo!("auto"),
            (_, Type::Forall { .. }) => todo!("forall"),

            // Var type
            (expr, Type::Var(var)) => {
                let expected = self
                    .type_aliases
                    .lookup(var)
                    .ok_or(Error::UndefinedTypeVariable(var.clone()))?;
                self.ensure(expr, expected)?
            }

            (Expr::Panic, _) => {}

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

            // Records
            (Expr::Record(actual_fields), Type::Record(expected_fields)) => {
                // TODO:
                // self.check_ty_wf(expected)?;
                check_no_duplicate_field_names(actual_fields)
                    .map_err(|field| Error::DuplicateRecordFields { expr, field })?;
                for Binding { name, value } in actual_fields {
                    self.ensure(
                        value,
                        expected_fields
                            .iter()
                            .find_map(|(expected_name, expected_ty)| {
                                if name == expected_name {
                                    Some(expected_ty)
                                } else {
                                    None
                                }
                            })
                            .ok_or(Error::UnexpectedRecordFields {
                                actual: expr,
                                expected: expected.to_owned(),
                                unexpected: Borrowed(name),
                            })?,
                    )?
                }

                for (needle, _) in expected_fields {
                    if !actual_fields
                        .iter()
                        .any(|Binding { name, value: _ }| name == needle)
                    {
                        throw!(Error::MissingRecordFields {
                            actual: expr,
                            expected: expected.to_owned(),
                            missing: needle.clone()
                        })
                    }
                }
            }
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

            // Variant
            (Expr::Variant { label, data }, Type::Variant(options)) => {
                let expected = options
                    .iter()
                    .find_map(|(name, data)| if name == label { Some(data) } else { None })
                    .ok_or(Error::UnexpectedVariantLabel {
                        actual: Borrowed(label),
                        expected: expected.to_owned(),
                    })?;
                match (data, expected) {
                    (None, None) => Ok(()),
                    (Some(actual), Some(expected)) => self.ensure(actual, expected),
                    (None, Some(expected)) => Err(Error::MissingDataForLabel {
                        label: Borrowed(label),
                        expected: expected.to_owned(),
                    }),
                    (Some(unexpected), None) => Err(Error::UnexpectedDataForNullaryLabel {
                        label: Borrowed(label),
                        unexpected,
                    }),
                }?
            }

            (
                Expr::Variant { .. },
                Type::Bool
                | Type::Nat
                | Type::Ref(_)
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::List(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedVariant {
                variant: expr,
                expected: expected.to_owned()
            }),

            // Seq
            (Expr::Seq(seq), expected) => {
                let [first, second] = seq.as_ref();
                self.ensure(first, &Type::Unit)?;
                self.ensure(second, expected)?
            }

            // ref
            (Expr::New(actual), Type::Ref(expected)) => self.ensure(actual, expected)?,
            (
                Expr::New(_),
                Type::Bool
                | Type::Nat
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedReference {
                reference: expr,
                expected: expected.to_owned()
            }),

            (Expr::Address(_), Type::Ref(_expected)) => {
                // TODO:
                // self.check_ty_wf(expected)?
            }
            (
                Expr::Address(_),
                Type::Bool
                | Type::Nat
                | Type::Sum { .. }
                | Type::Fn { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
            ) => throw!(Error::UnexpectedMemoryAddress {
                address: expr,
                expected: expected.to_owned()
            }),

            // fallback to infer + check_ty_coerces
            (
                Expr::Var(_)
                | Expr::Unit
                | Expr::True
                | Expr::False
                | Expr::Integer(_)
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
            )
            | (_, Type::Bot | Type::Top) => {
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
            (Expr::Assign { .. }, Type::Bool)
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
            | (Expr::TryCastAs { .. }, Type::Unit { .. }) => {
                todo!("typck {expr:?} is {expected:?}")
            }
        }
        Ok(())
    }
}
