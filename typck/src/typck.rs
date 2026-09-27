use std::{
    borrow::Cow::{self, Borrowed},
    collections::{HashMap, HashSet},
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

type Namespace<'ctx, 'src> = crate::namespace::Namespace<'ctx, Ident<'src>, Cow<'src, Type<'src>>>;

enum ExceptionType<'ctx, 'src> {
    None,
    FromParent(&'ctx ExceptionType<'ctx, 'src>),
    Simple(&'src Type<'src>),
    Open(HashMap<Ident<'src>, &'src Type<'src>>),
}

struct Context<'ctx, 'src> {
    type_aliases: Namespace<'ctx, 'src>,
    variables: Namespace<'ctx, 'src>,
    exception_type: ExceptionType<'ctx, 'src>,
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

#[throws(Ident<'src>)]
fn check_no_duplicate_field_names<'src>(fields: &[Binding<'src>]) {
    let mut seen = HashSet::<&'src str>::new();
    for Binding { name, value: _ } in fields {
        if !seen.insert(name.as_ref()) {
            throw!(name.clone())
        }
    }
}

impl<'src> Context<'_, 'src> {
    #[throws(Error<'src>)]
    fn check_program(program: &'src Program<'src>) {
        Self {
            type_aliases: Namespace::empty(),
            variables: Namespace::empty(),
            exception_type: ExceptionType::None,
            ambiguous_type_as_bottom: program
                .extensions
                .contains(&Extension::AmbiguousTypeAsBottom),
            structural_subtyping: program.extensions.contains(&Extension::StructuralSubtyping),
        }
        .check_decls(program.decls.iter())?
    }

    #[must_use]
    fn new_scope<'this>(&'this self) -> Context<'this, 'src> {
        let &Self {
            ref type_aliases,
            ref variables,
            ref exception_type,
            ambiguous_type_as_bottom,
            structural_subtyping,
        } = self;
        Context {
            type_aliases: type_aliases.new_scope(),
            variables: variables.new_scope(),
            exception_type: ExceptionType::FromParent(exception_type),
            ambiguous_type_as_bottom,
            structural_subtyping,
        }
    }
}

#[throws(Error<'src>)]
pub(crate) fn typck<'src>(program: &'src Program<'src>) {
    gates::check_extensions(program)?;
    check_main(program)?;

    Context::check_program(program)?;
}
