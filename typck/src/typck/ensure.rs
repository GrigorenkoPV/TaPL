use core::iter::zip;
use std::borrow::Cow::{Borrowed, Owned};

use culpa::throw;

use crate::{
    ast::{Binding, Expr, Type},
    extensions::{Extension, unsupported},
    typck::error::RecordWithMissingFields,
};

use super::{Context, Error, check_no_duplicate_field_names, nat_rec_step_type};

impl<'src> Context<'_, 'src> {
    #[expect(clippy::too_many_lines, reason = "giant match")]
    pub(super) fn ensure(
        &self,
        expr: &'src Expr<'src>,
        expected: &Type<'src>,
    ) -> Result<(), Error<'src>> {
        match (expr, expected) {
            // Unsupported
            (e @ (Expr::Fold { .. } | Expr::Unfold { .. }), _) => unsupported::fold_unfold(e),
            (_, t @ Type::Rec { .. }) => unsupported::mu(t),

            // Unknown stage
            (
                Expr::BinOp { .. },
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
                const { debug_assert!(!Extension::ArithmeticOperators.supported(), "TODO") }
                unreachable!("ensure {expr:?} is {expected:?}")
            }
            // Stage 1 (optional)
            (Expr::Let { rec: true, .. }, _) => {
                const { debug_assert!(!Extension::LetrecBindings.supported(), "TODO") }
                unreachable!("stage 1: ensure {expr:?} is {expected:?}")
            }

            // Stage 2 (optional)
            (
                Expr::TryCastAs { .. },
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
                const { debug_assert!(!Extension::TryCastAs.supported(), "TODO") }
                unreachable!("stage 2: ensure {expr:?} is {expected:?}")
            }

            // Stage 3
            (
                Expr::Generic { .. } | Expr::TypeApplication { .. },
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
            | (_, Type::Forall { .. }) => {
                const { debug_assert!(!Extension::UniversalTypes.supported(), "TODO") }
                unreachable!("stage 3: ensure {expr:?} is {expected:?}")
            }
            (_, Type::Auto) => {
                const { debug_assert!(!Extension::TypeReconstruction.supported(), "TODO") }
                unreachable!("stage 3: ensure {expr:?} is {expected:?}")
            }

            // TypeAliases
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
                    rec: false,
                    bindings,
                    body,
                },
                ty,
            ) => {
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
                    if self.structural_subtyping {
                        self.check_subtype(actual, expected, None)?
                    } else if expected != actual {
                        throw!(Error::UnexpectedTypeForParameter {
                            param: Borrowed(name),
                            actual: actual.to_owned(),
                            expected: expected.to_owned(),
                        })
                    }
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
                | Type::Nat
                | Type::Ref(_)
                | Type::Sum { .. }
                | Type::Tuple(_)
                | Type::Record(_)
                | Type::Variant(_)
                | Type::List(_)
                | Type::Unit,
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
                    match expected_fields
                        .iter()
                        .find_map(|(expected_name, expected_ty)| {
                            if name == expected_name {
                                Some(expected_ty)
                            } else {
                                None
                            }
                        }) {
                        Some(expected_ty) => self.ensure(value, expected_ty)?,
                        None => {
                            if self.structural_subtyping {
                                let inferred = self.infer(value)?;
                                drop(inferred);
                            } else {
                                throw!(Error::UnexpectedRecordFields {
                                    actual: expr,
                                    expected: expected.to_owned(),
                                    unexpected: Borrowed(name),
                                })
                            }
                        }
                    }
                }

                for (needle, _) in expected_fields {
                    if !actual_fields
                        .iter()
                        .any(|Binding { name, value: _ }| name == needle)
                    {
                        throw!(Error::MissingRecordFields {
                            actual: RecordWithMissingFields::Literal(expr),
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

            (Expr::Match { switch, cases }, expected) => {
                let switch_actual = self.infer(switch)?;
                todo!("check match (_ : {switch_actual}) {cases:?} : {expected}")
            }

            // Seq
            (Expr::Seq(seq), expected) => {
                let [first, second] = seq.as_ref();
                self.ensure(first, &Type::Unit)?;
                self.ensure(second, expected)?
            }

            // ref creation
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

            // ref consumption
            (Expr::Deref(inner), expected) => {
                match self.ensure(
                    inner,
                    &Type::Ref(
                        // FIXME: `Rc` / `Cow`
                        Box::new(expected.to_owned()),
                    ),
                ) {
                    // References are invariant, but dereferencing is covariant
                    Err(_) if self.structural_subtyping => {
                        let actual = self.infer(inner)?;
                        let Type::Ref(actual) = actual.as_ref() else {
                            throw!(Error::NotAReference {
                                expr,
                                actual: actual.into_owned()
                            })
                        };
                        self.check_subtype(actual, expected, Some(expr))?
                    }
                    otherwise => otherwise?,
                }
            }

            (Expr::Cast { expr: inner, ty }, expected) => {
                let actual = self.infer(inner)?;
                drop(actual);
                self.check_coe(Borrowed(ty), expected, expr)?
            }

            (Expr::Throw(exc), _) => {
                let exception_type = self.get_exception_type()?;
                self.ensure(exc, &exception_type)?;
            }

            (Expr::TryWith { r#try, with }, expected) => {
                self.ensure(r#try, expected)?;
                self.ensure(with, expected)?;
            }

            (
                Expr::TryCatch { .. },
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
                todo!("ensure {expr:?} is {expected:?}")
            }

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
                | Expr::DotTuple { .. }
                | Expr::Assign { .. },
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
                self.check_coe(actual, expected, expr)?
            }
        }
        Ok(())
    }
}
