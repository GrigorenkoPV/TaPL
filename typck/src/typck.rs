use core::marker::PhantomData;

use culpa::{throw, throws};

use crate::ast::{Decl, Function, Program};

mod error;
mod gates;

pub(crate) use self::error::Error;

struct Context<'ctx> {
    data_: PhantomData<&'ctx Program<'ctx>>,
}

impl<'ctx> Context<'ctx> {
    #[must_use]
    const fn new() -> Self {
        Self { data_: PhantomData }
    }

    #[throws(Error<'ctx>)]
    fn check_program(&self, program: &Program<'ctx>) {
        for decl in &program.decls {
            todo!("Check {decl:?}")
        }
    }
}

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
        [one, two, ..] => {
            throw!(Error::DuplicateFunctionDeclaration([one, two]))
        }
        [main] => {
            let arity = main.params.len();
            if arity != 1 {
                throw!(Error::IncorrectArityOfMain(arity))
            }
        }
    }
}

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    gates::check_extensions(program)?;
    check_main(program)?;
    Context::new().check_program(program)?;
}
