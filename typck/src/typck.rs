use culpa::{throw, throws};

use crate::ast::{Decl, Function, Ident, Program};

mod error;
mod gates;

pub(crate) use self::error::Error;

struct Context<'ctx> {
    program: &'ctx Program<'ctx>,
}

impl<'ctx> Context<'ctx> {
    #[throws(Error<'ctx>)]
    fn find_global(&self, name: &Ident<'_>) -> Option<&'ctx Decl<'ctx>> {
        let mut decls = self
            .program
            .decls
            .iter()
            .filter(move |decl| decl.name() == Some(name));
        let Some(found) = decls.next() else {
            return None;
        };
        if let Some(other) = decls.next() {
            throw!(Error::DuplicateFunctionDeclaration([found, other]))
        }
        Some(found)
    }

    #[throws(Error<'ctx>)]
    fn find_global_function(&self, name: &Ident<'_>) -> Option<&'ctx Function<'ctx>> {
        self.find_global(name)?.and_then(|decl| match decl {
            Decl::Function(function) => Some(function.as_ref()),
            Decl::TypeAlias { .. } | Decl::ExceptionType(_) | Decl::ExceptionVariant { .. } => None,
        })
    }

    #[throws(Error<'ctx>)]
    fn check_main(&self) {
        let Function { params, .. } = self
            .find_global_function(&"main".into())?
            .ok_or(Error::MissingMain)?;
        let arity = params.len();
        if arity != 1 {
            throw!(Error::IncorrectArityOfMain(arity))
        }
    }

    #[throws(Error<'ctx>)]
    fn check_global_decl(&self, decl: &Decl<'ctx>) {
        todo!("Check {decl:?}")
    }
}

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    gates::check_extensions(program)?;
    let ctx = Context { program };
    ctx.check_main()?;
    for decl in &program.decls {
        ctx.check_global_decl(decl)?
    }
}
