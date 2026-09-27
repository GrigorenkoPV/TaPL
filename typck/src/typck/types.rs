use core::iter::zip;

use culpa::throw;

use crate::{
    ast::{Expr, Type},
    extensions::{Extension, unsupported},
};

use super::{Context, Error};

impl<'ctx> Context<'_, 'ctx> {
    #[expect(clippy::too_many_lines, reason = "big switch")]
    pub(super) fn check_subtype(
        &self,
        actual: &Type<'ctx>,
        expected: &Type<'ctx>,
        expr: Option<&'ctx Expr<'ctx>>,
    ) -> Result<(), Error<'ctx>> {
        debug_assert!(
            self.structural_subtyping,
            "use equality if there's no subtyping"
        );

        let error = || Error::UnexpectedSubtype {
            expected: expected.to_owned(),
            actual: actual.to_owned(),
            expr,
        };

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

            [Type::Record(from), Type::Record(to)] => {
                todo!("subtyping of records")
            }

            [Type::Variant(from), Type::Variant(to)] => {
                todo!("subtyping of variants")
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
