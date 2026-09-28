use std::borrow::Cow::{self, Borrowed};

use culpa::throws;

use crate::{
    ast::{Pattern, PatternBinding, Type},
    extensions::Extension,
};

use super::{Context, Error};

impl<'src> Context<'_, 'src> {
    #[throws(Error<'src>)]
    pub(super) fn check_bindings(
        &mut self,
        bindings: impl IntoIterator<Item = &'src PatternBinding<'src>>,
    ) {
        for binding in bindings {
            self.check_binding(binding)?
        }
    }

    #[throws(Error<'src>)]
    fn check_binding(&mut self, binding: &'src PatternBinding<'src>) {
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
            | Pattern::Succ(_) => {
                const { debug_assert!(!Extension::LetPatterns.supported(), "TODO") };
                unreachable!()
            }
        }
    }

    #[throws(Error<'src>)]
    pub(super) fn bind(&mut self, pat: &'src Pattern<'src>, ty: Cow<'src, Type<'src>>) {
        match pat {
            Pattern::Var(name) => self
                .variables
                .add_local(Borrowed(name), ty)
                .map_err(|(_old, _new)| todo!())?,
            Pattern::CastAs { .. }
            | Pattern::Unit
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
            | Pattern::Succ(_) => todo!(),
        }
    }
}
