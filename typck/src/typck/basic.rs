use core::mem;
use std::{
    borrow::Cow::{Borrowed, Owned},
    collections::HashSet,
};

use culpa::{throw, throws};

use crate::{
    ast::{Decl, Function, Ident, Type},
    extensions::unsupported,
};

use super::{Context, Error, ExceptionType};

impl<'src> Context<'_, 'src> {
    #[throws(Error<'src>)]
    pub(super) fn check_decls(&mut self, decls: impl IntoIterator<Item = &'src Decl<'src>>) {
        for decl in decls {
            self.check_decl(decl)?
        }
    }

    #[throws(Error<'src>)]
    fn check_decl(&mut self, decl: &'src Decl<'src>) {
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
            Decl::ExceptionType(ty) => {
                match mem::replace(&mut self.exception_type, ExceptionType::Simple(ty)) {
                    ExceptionType::None | ExceptionType::FromParent(_) => {}
                    ExceptionType::Simple(_) => todo!("report redefinition of exception type"),
                    ExceptionType::Open(_) => todo!("report conflicted exception type"),
                }
            }
            Decl::ExceptionVariant { label, data } => {
                let old_label = label;
                let label: Ident<'src> = Borrowed(old_label);
                match &mut self.exception_type {
                    overridable @ (ExceptionType::None | ExceptionType::FromParent(_)) => {
                        *overridable = ExceptionType::Open([(label, data)].into())
                    }
                    ExceptionType::Simple(_) => todo!("report conflicted exception type"),
                    ExceptionType::Open(variants) => {
                        if let Some(prev) = variants.insert(label, data) {
                            todo!("report redefinition of exception variant: {old_label} = {prev}")
                        }
                    }
                }
            }
        }
    }

    #[throws(Error<'src>)]
    fn check_ty_wf(&self, ty: &'src Type<'src>) {
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

            t @ Type::Rec { .. } => unsupported::mu(t),
        }
    }

    #[throws(Error<'src>)]
    fn check_function(&mut self, function: &'src Function<'src>) {
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
        ctx.ensure(body, ret_ty)?
    }
}
