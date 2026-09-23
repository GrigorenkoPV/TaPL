use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::HashSet,
};

use culpa::{throw, throws};

use crate::ast::{Decl, Expr, Function, Ident, Program, Type};

mod error;
mod gates;

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
        [] => throw!(Error::MissingMain),
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
}

impl<'ctx> Context<'_, 'ctx> {
    #[throws(Error<'ctx>)]
    fn check_program(program: &'ctx Program<'ctx>) {
        Self {
            type_aliases: Namespace::empty(),
            variables: Namespace::empty(),
        }
        .check_decls(program.decls.iter())?
    }

    #[must_use]
    fn new_scope<'this>(&'this self) -> Context<'this, 'ctx> {
        let Self {
            type_aliases,
            variables,
        } = self;
        Context {
            type_aliases: type_aliases.new_scope(),
            variables: variables.new_scope(),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_decls(&mut self, decls: impl IntoIterator<Item = &'ctx Decl<'ctx>>) {
        for decl in decls {
            self.check_decl(decl)?
        }
    }

    #[throws(Error<'ctx>)]
    fn check_decl(&mut self, decl: &'ctx Decl<'ctx>) {
        match decl {
            Decl::Function(function) => self.check_function(function)?,
            Decl::TypeAlias { name, ty } => {
                self.check_ty_wf(ty)?;
                self.type_aliases
                    .add_local(Borrowed(name), Borrowed(ty))
                    .map_err(|(new, old)| {
                        todo!("Duplicate type alias {name:?}: old: {old:?}, new: {new:?}")
                    })?
            }
            Decl::ExceptionType(_) | Decl::ExceptionVariant { .. } => todo!(),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_ty_wf(&self, ty: &'ctx Type<'ctx>) {
        match ty {
            Type::Bool | Type::Nat | Type::Unit | Type::Top | Type::Bot | Type::Auto => {}

            Type::Ref(inner) | Type::List(inner) => self.check_ty_wf(inner)?,

            Type::Sum { left, right } => {
                self.check_ty_wf(left)?;
                self.check_ty_wf(right)?;
            }
            Type::Tuple(inner) => {
                for t in inner {
                    self.check_ty_wf(t)?
                }
            }
            Type::Forall { binders, ty } => {
                let mut ctx = self.new_scope();
                for name in binders {
                    ctx.type_aliases
                        .add_local(Borrowed(name), Owned(Type::Auto))
                        .map_err(|(_old, _new)| Error::DuplicateTypeParameter(Borrowed(name)))?;
                }
                ctx.check_ty_wf(ty)?
            }

            Type::Fn { args, ret } => {
                for arg in args {
                    self.check_ty_wf(arg)?
                }
                self.check_ty_wf(ret)?
            }

            Type::Record(fields) => {
                let mut names = HashSet::<&str>::new();
                for (name, ty) in fields {
                    if !names.insert(name) {
                        throw!(Error::DuplicateRecordTypeFields(Borrowed(name)))
                    }
                    self.check_ty_wf(ty)?
                }
            }
            Type::Variant(variants) => {
                let mut names = HashSet::<&str>::new();
                for (name, ty) in variants {
                    if !names.insert(name) {
                        throw!(Error::DuplicateVariantTypeFields(Borrowed(name)))
                    }
                    if let Some(ty) = ty {
                        self.check_ty_wf(ty)?
                    }
                }
            }

            Type::Var(name) => match self.type_aliases.lookup(name) {
                Some(_) => {}
                None => throw!(Error::UndefinedTypeVariable(Borrowed(name))),
            },

            Type::Rec { .. } => unimplemented!(),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_function(&mut self, function: &'ctx Function<'ctx>) {
        let Function {
            annotations,
            name: fn_name,
            generics,
            params,
            ret_ty,
            throws,
            decls,
            body,
        } = function;
        assert!(
            annotations.is_empty(),
            "Functions with annotations are not supported"
        );
        if !generics.is_empty() {
            todo!()
        }
        assert!(throws.is_empty(), "Functions that throw are not supported");
        let ret_ty = ret_ty
            .as_ref()
            .expect("Function without a return type are not supported");

        self.check_ty_wf(ret_ty)?;
        self.variables
            .add_local(
                Borrowed(fn_name),
                Owned(Type::Fn {
                    args: params
                        .iter()
                        .map(
                            |(_name, ty)| ty.clone(), // FIXME: use `Cow`?
                        )
                        .collect(),
                    ret: Box::new(ret_ty.clone()), // FIXME: use `Cow`?
                }),
            )
            .map_err(|(_old, _new)| Error::DuplicateFunctionDeclaration(Borrowed(fn_name)))?;

        let mut ctx = self.new_scope();
        for (name, ty) in params {
            self.check_ty_wf(ty)?;
            ctx.variables
                .add_local(Borrowed(name), Borrowed(ty))
                .map_err(|(_old, _new)| Error::DuplicateFunctionParameter {
                    function: Borrowed(fn_name),
                    param: Borrowed(name),
                })?
        }
        ctx.check_decls(decls)?;
        ctx.check_expr_is_ty(body, ret_ty)?
    }

    #[throws(Error<'ctx>)]
    fn check_expr_is_ty(&self, expr: &'ctx Expr<'ctx>, ty: &Type<'_>) {
        match (expr, ty) {
            (Expr::False | Expr::True, Type::Bool)
            | (Expr::Unit, Type::Unit)
            | (Expr::Integer(_), Type::Nat) => {}

            (Expr::Var(var), expected) => {
                let actual = self
                    .variables
                    .lookup(var)
                    .ok_or(Error::UndefinedVariable(Borrowed(var)))?;
                self.check_ty_coerces(actual, expected)?
            }

            (expr, ty) => todo!("typck {expr:?} is {ty:?}"),
        }
    }

    #[throws(Error<'ctx>)]
    fn check_ty_coerces(&self, from: &Type<'_>, to: &Type<'_>) {
        match [from, to] {
            [Type::Bool, Type::Bool] | [Type::Nat, Type::Nat] | [Type::Unit, Type::Unit] => {}
            [from, to] => todo!("Check coercion from {from:?} to {to:?}"),
        }
    }
}

#[throws(Error<'ctx>)]
pub(crate) fn typck<'ctx>(program: &'ctx Program<'ctx>) {
    gates::check_extensions(program)?;
    check_main(program)?;

    Context::check_program(program)?;
}
