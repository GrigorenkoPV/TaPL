use core::iter::zip;
use std::borrow::Cow::{self, Borrowed, Owned};

use culpa::throw;

use crate::{
    ast::{Binding, Expr, Integer, Type},
    extensions::{Extension, unsupported},
};

use super::{Context, Error, check_no_duplicate_field_names, nat_rec_step_type};

fn dot_tuple(types: Vec<Type<'_>>, index: Integer) -> Option<Type<'_>> {
    let index = index.checked_sub(1)?.try_into().ok()?;
    types.into_iter().nth(index)
}

impl<'src> Context<'_, 'src> {
    #[expect(clippy::too_many_lines, reason = "big match")]
    pub(super) fn infer(
        &self,
        expr: &'src Expr<'src>,
    ) -> Result<Cow<'src, Type<'src>>, Error<'src>> {
        Ok(match expr {
            // Unsupported:

            // Unknown stage
            Expr::BinOp { .. } => {
                const { debug_assert!(!Extension::ArithmeticOperators.supported(), "TODO") }
                unreachable!("infer {expr:?}")
            }

            // Stage 1
            Expr::Match { .. } => {
                todo!("infer match")
            }

            // Stage 1 (optional)
            Expr::Let { rec: true, .. } => {
                const { debug_assert!(!Extension::LetrecBindings.supported(), "TODO") }
                unreachable!("stage 1: infer {expr:?}")
            }

            Expr::Throw(_) | Expr::TryWith { .. } | Expr::TryCatch { .. } => {
                const { debug_assert!(!Extension::Exceptions.supported(), "TODO") }
                unreachable!("stage 2: infer {expr:?}")
            }

            // Stage 2 (optional)
            Expr::TryCastAs { .. } => {
                const { debug_assert!(!Extension::TryCastAs.supported(), "TODO") }
                unreachable!("stage 2: infer {expr:?}")
            }

            // Stage 3
            Expr::Generic { .. } | Expr::TypeApplication { .. } => {
                const { debug_assert!(!Extension::UniversalTypes.supported(), "TODO") }
                unreachable!("stage 3: infer {expr:?}")
            }

            Expr::Fold { .. } | Expr::Unfold { .. } => unsupported::fold_unfold(expr),

            Expr::True | Expr::False => Owned(Type::Bool),
            Expr::Unit => Owned(Type::Unit),
            Expr::Integer(_) => Owned(Type::Nat),

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
                rec: false,
                bindings,
                body,
            } => {
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
                if arg_ty != ret.as_ref() {
                    throw!(Error::UnexpectedTypeForExpression {
                        expr,
                        expected: Type::Fn {
                            args: vec![ret.as_ref().to_owned()],
                            ret: ret.clone()
                        },
                        actual: Type::Fn { args, ret },
                    })
                }
                Owned(*ret)
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
            Expr::Cast { expr, ty } => {
                let inferred = self.infer(expr)?;
                drop(inferred);
                Borrowed(ty)
            }

            Expr::Inl(inner) => {
                if self.ambiguous_type_as_bottom {
                    Owned(Type::Sum {
                        left: Box::new(self.infer(inner)?.into_owned()),
                        right: Box::new(Type::Bot),
                    })
                } else {
                    throw!(Error::AmbiguousSumType(expr))
                }
            }
            Expr::Inr(expr) => {
                if self.ambiguous_type_as_bottom {
                    Owned(Type::Sum {
                        left: Box::new(Type::Bot),
                        right: Box::new(self.infer(expr)?.into_owned()),
                    })
                } else {
                    throw!(Error::AmbiguousSumType(expr))
                }
            }
            Expr::Variant { .. } => throw!(Error::AmbiguousVariantType(expr)),

            Expr::List(elements) => match elements.as_slice() {
                [] => {
                    if self.ambiguous_type_as_bottom {
                        Owned(Type::List(Box::new(Type::Bot)))
                    } else {
                        throw!(Error::AmbiguousList {})
                    }
                }
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

            Expr::New(expr) => Owned(Type::Ref(Box::new(self.infer(expr)?.into_owned()))),
            Expr::Address(_) => throw!(Error::AmbiguousReferenceType {}),
            Expr::Deref(expr) => {
                let actual = self.infer(expr)?.into_owned();
                let Type::Ref(result) = actual else {
                    throw!(Error::NotAReference { expr, actual })
                };
                Owned(*result)
            }
            Expr::Assign { lhs, rhs } => {
                let ref_t = self.infer(lhs)?;
                let Type::Ref(expected) = ref_t.as_ref() else {
                    throw!(Error::NotAReference {
                        expr: lhs,
                        actual: ref_t.into_owned(),
                    })
                };
                self.ensure(rhs, expected)?;
                Owned(Type::Unit)
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
            Expr::Seq(seq) => {
                let [first, second] = seq.as_ref();
                self.ensure(first, &Type::Unit)?;
                self.infer(second)?
            }

            Expr::Panic => {
                if self.ambiguous_type_as_bottom {
                    Owned(Type::Bot)
                } else {
                    throw!(Error::AmbiguousPanicType {})
                }
            }
        })
    }
}
