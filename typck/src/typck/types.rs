use core::iter::zip;
use std::borrow::Cow;

use culpa::{throw, throws};

use crate::{
    ast::{Expr, Type},
    extensions::{Extension, unsupported},
};

use super::{
    Context,
    error::{Error, RecordWithMissingFields},
};

impl<'src> Context<'_, 'src> {
    #[throws(Error<'src>)]
    pub(super) fn check_coe(
        &self,
        actual: Cow<'src, Type<'src>>,
        expected: &Type<'src>,
        expr: &'src Expr<'src>,
    ) {
        if self.structural_subtyping {
            self.check_subtype(actual.as_ref(), expected, Some(expr))?
        } else if expected != actual.as_ref() {
            throw!(Error::UnexpectedTypeForExpression {
                expr,
                actual: actual.into_owned(),
                expected: expected.to_owned(),
            })
        }
    }

    #[expect(clippy::too_many_lines, reason = "big switch")]
    pub(super) fn check_subtype(
        &self,
        actual: &Type<'src>,
        expected: &Type<'src>,
        expr: Option<&'src Expr<'src>>,
    ) -> Result<(), Error<'src>> {
        debug_assert!(
            self.structural_subtyping,
            "use equality if there's no subtyping"
        );

        let error = || Error::UnexpectedSubtype {
            expected: expected.to_owned(),
            actual: actual.to_owned(),
            expr,
        };

        let actual_expr = expr;
        #[expect(
            unused,
            reason = "prevent the function argument `expr` from being used
            anywhere beside error creation at this level of recursion"
        )]
        let expr = ();

        match [actual, expected] {
            // Unsupported
            [rec @ Type::Rec { .. }, _] | [_, rec @ Type::Rec { .. }] => unsupported::mu(rec),

            [Type::Var(_), _] | [_, Type::Var(_)] => {
                debug_assert!(!Extension::UniversalTypes.supported(), "TODO");
                unreachable!()
            }

            // Trivial
            [Type::Bot, _] | [_, Type::Top] => Ok(()),

            // Covariant
            [Type::List(from), Type::List(to)] => self.check_subtype(from, to, None),
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
                self.check_subtype(from_left, to_left, None)?;
                self.check_subtype(from_right, to_right, None)?;
                Ok(())
            }
            [Type::Tuple(from), Type::Tuple(to)] => {
                if from.len() != to.len() {
                    throw!(error())
                }
                for (from, to) in zip(from, to) {
                    self.check_subtype(from, to, None)?
                }
                Ok(())
            }

            // Invariant
            [Type::Ref(from), Type::Ref(to)] => {
                self.check_subtype(from, to, None)?;
                self.check_subtype(to, from, None)?;
                Ok(())
            }

            // Contravariant in arguments, covariant in return type
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
                    throw!(error())
                }
                for (arg_from, arg_to) in zip(args_from, args_to) {
                    self.check_subtype(arg_to, arg_from, None)?
                }
                self.check_subtype(ret_from, ret_to, None)
            }

            [Type::Record(actual_fields), Type::Record(expected_fields)] => {
                for (expected_name, expected_field) in expected_fields {
                    let Some(actual_field) = actual_fields.iter().find_map(|(name, ty)| {
                        if name == expected_name {
                            Some(ty)
                        } else {
                            None
                        }
                    }) else {
                        throw!(Error::MissingRecordFields {
                            actual: RecordWithMissingFields::Subtyping {
                                actual_expr,
                                actual_type: actual.to_owned()
                            },
                            expected: expected.to_owned(),
                            missing: expected_name.clone(),
                        })
                    };
                    self.check_subtype(actual_field, expected_field, None)?
                }
                Ok(())
            }

            [
                Type::Variant(actual_variant),
                Type::Variant(expected_variant),
            ] => {
                for (actual_label, actual_data) in actual_variant {
                    let Some(expected_data) = expected_variant.iter().find_map(|(label, data)| {
                        if label == actual_label {
                            Some(data)
                        } else {
                            None
                        }
                    }) else {
                        throw!(Error::UnexpectedVariantLabel {
                            expected: expected.clone(),
                            actual: actual_label.clone()
                        })
                    };
                    match [actual_data, expected_data] {
                        [Some(_) | None, None] => Ok(()),
                        [None, Some(to)] => Err(Error::MissingDataForLabel {
                            label: actual_label.clone(),
                            expected: to.clone(),
                        }),
                        [Some(from), Some(to)] => self.check_subtype(from, to, None),
                    }?
                }
                Ok(())
            }

            // No special handling
            [from, to] => {
                if from == to {
                    Ok(())
                } else {
                    Err(error())
                }
            }
        }
    }
}
