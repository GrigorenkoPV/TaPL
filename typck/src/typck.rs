use std::{
    borrow::Cow::{self, Borrowed},
    collections::HashSet,
};

use culpa::{throw, throws};

use crate::{
    ast::{Binding, Decl, Function, Ident, Program, Type},
    extensions::Extension,
};

mod basic;
mod ensure;
mod error;
mod gates;
mod infer;
mod patterns;
mod types;

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
        [] => throw!(Error::MissingMain {}),
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
    ambiguous_type_as_bottom: bool,
    structural_subtyping: bool,
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
            ambiguous_type_as_bottom: program
                .extensions
                .contains(&Extension::AmbiguousTypeAsBottom),
            structural_subtyping: program.extensions.contains(&Extension::StructuralSubtyping),
        }
        .check_decls(program.decls.iter())?
    }

    #[must_use]
    fn new_scope<'this>(&'this self) -> Context<'this, 'ctx> {
        let &Self {
            ref type_aliases,
            ref variables,
            ambiguous_type_as_bottom,
            structural_subtyping,
        } = self;
        Context {
            type_aliases: type_aliases.new_scope(),
            variables: variables.new_scope(),
            ambiguous_type_as_bottom,
            structural_subtyping,
        }
    }
}

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    gates::check_extensions(program)?;
    check_main(program)?;

    Context::check_program(program)?;
}
