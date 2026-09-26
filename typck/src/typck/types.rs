use core::{iter::zip, mem};

use culpa::throw;

use crate::{ast::Type, extensions::unsupported};

use super::Context;

#[expect(
    clippy::self_only_used_in_recursion,
    single_use_lifetimes,
    clippy::elidable_lifetime_names,
    reason = "WIP"
)]
impl<'ctx> Context<'_, 'ctx> {
    // TODO: is this just equality?
    pub(super) fn check_ty_coerces(&self, from: &Type<'_>, to: &Type<'_>) -> Result<(), ()> {
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

            [Type::Tuple(from), Type::Tuple(to)] => {
                if from.len() != to.len() {
                    throw!(())
                }
                for (from, to) in zip(from, to) {
                    self.check_ty_coerces(from, to)?
                }
                Ok(())
            }
            [Type::Record(from), Type::Record(to)] => {
                if from.len() != to.len() {
                    throw!(())
                }
                for ((from_name, from), (to_name, to)) in zip(from, to) {
                    if from_name != to_name {
                        throw!(())
                    }
                    self.check_ty_coerces(from, to)?
                }
                Ok(())
            }

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
