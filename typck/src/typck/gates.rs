use core::mem;

use crate::{
    ast::{
        Annotation, Binding, Catch, Decl, Expr, Function, HasVisitable, Pattern, PatternBinding,
        Program, Type, Visitor,
    },
    extensions::{Extension, Extensions},
};

struct GateCollector {
    required: Extensions,
    is_in_global_scope: bool,
}

impl GateCollector {
    #[must_use]
    const fn new() -> Self {
        Self {
            required: Extensions::new(),
            is_in_global_scope: true,
        }
    }
    fn require(&mut self, extension: Extension) {
        let _: bool = self.required.insert(extension);
    }

    fn require_fn_arity(&mut self, arity: usize) {
        match arity {
            0 => self.require(Extension::NullaryFunctions),
            1 => {}
            _ => self.require(Extension::MultiparameterFunctions),
        }
    }
}

impl<'ctx> Visitor<'ctx> for GateCollector {
    fn visit_function(&mut self, function: &Function<'ctx>) {
        let Function {
            annotations: _,
            name: _,
            generics,
            params,
            ret_ty,
            throws,
            decls: _,
            body: _,
        } = function;

        if !generics.is_empty() {
            todo!()
        }
        self.require_fn_arity(params.len());
        if ret_ty.is_none() {
            todo!()
        }
        if !throws.is_empty() {
            todo!()
        }

        let was_in_global_scope = mem::replace(&mut self.is_in_global_scope, false);
        function.visit_inner_with(self);
        self.is_in_global_scope = was_in_global_scope;
    }

    fn visit_annotation(&mut self, annotation: &Annotation) {
        match annotation {
            Annotation::Inline => todo!(),
        }
    }

    fn visit_pattern_binding(&mut self, pattern_binding: &PatternBinding<'ctx>) {
        let PatternBinding { lhs, rhs: _ } = pattern_binding;
        if let Pattern::Var(_) = lhs {
        } else {
            self.require(Extension::LetPatterns);
        }
        pattern_binding.visit_inner_with(self)
    }

    fn visit_binding(&mut self, binding: &Binding<'ctx>) {
        self.require(Extension::Records);
        binding.visit_inner_with(self)
    }

    fn visit_decl(&mut self, decl: &Decl<'ctx>) {
        match decl {
            Decl::Function(_) => {
                if !self.is_in_global_scope {
                    self.require(Extension::NestedFunctionDeclarations)
                }
            }

            Decl::ExceptionType(_) => {
                self.require(Extension::Exceptions);
                self.require(Extension::ExceptionTypeDeclaration)
            }
            Decl::ExceptionVariant { .. } => self.require(Extension::OpenVariantExceptions),

            Decl::TypeAlias { name, ty } => todo!(),
        }
        decl.visit_inner_with(self)
    }

    fn visit_ty(&mut self, ty: &Type<'ctx>) {
        match ty {
            Type::Bool | Type::Nat => {}
            Type::Fn { args, ret: _ } => self.require_fn_arity(args.len()),
            Type::Unit => self.require(Extension::UnitType),
            Type::Tuple(_) => todo!("Is this `pairs` or `tuples`?"),
            Type::Record(_) => self.require(Extension::Records),
            Type::Sum { .. } => self.require(Extension::SumTypes),
            Type::List(_) => self.require(Extension::Lists),
            Type::Variant(variants) => {
                self.require(Extension::Variants);
                for (_, ty) in variants {
                    if ty.is_none() {
                        self.require(Extension::NullaryVariantLabels);
                    }
                }
            }

            Type::Ref(_) => self.require(Extension::References),
            Type::Top => self.require(Extension::TopType),
            Type::Bot => self.require(Extension::BottomType),

            Type::Forall { binders, ty } => todo!(),
            Type::Rec { var, ty } => todo!(),
            Type::Auto => todo!(),
            Type::Var(cow) => todo!(),
        }
        ty.visit_inner_with(self)
    }

    fn visit_pat(&mut self, pat: &Pattern<'ctx>) {
        match pat {
            Pattern::Var(_) => {
                todo!("`let-bindings` or `some-types`")
            }
            Pattern::Inl(_) | Pattern::Inr(_) => self.require(Extension::SumTypes),
            Pattern::Variant { label: _, pat } => {
                self.require(Extension::Variants);
                if pat.is_none() {
                    self.require(Extension::NullaryVariantLabels);
                }
            }
            Pattern::Tuple(_)
            | Pattern::Record(_)
            | Pattern::List(_)
            | Pattern::Cons(_)
            | Pattern::Int(_)
            | Pattern::Succ(_)
            | Pattern::False
            | Pattern::True
            | Pattern::Unit => self.require(Extension::StructuralPatterns),
            Pattern::Asc { .. } => self.require(Extension::PatternAscriptions),

            Pattern::CastAs { .. } => self.require(Extension::TypeCastPatterns),
        }
        pat.visit_inner_with(self)
    }

    fn visit_catch(&mut self, catch: &Catch<'ctx>) {
        todo!();
        catch.visit_inner_with(self)
    }

    fn visit_expr(&mut self, expr: &Expr<'ctx>) {
        match expr {
            Expr::True
            | Expr::False
            | Expr::If { .. }
            | Expr::Succ(_)
            | Expr::NatIsZero(_)
            | Expr::NatRec(_)
            | Expr::Var(_) => {}
            &Expr::Integer(i) => {
                if i != 0 {
                    self.require(Extension::NaturalLiterals)
                }
            }
            Expr::Abstraction { params, ret: _ } => self.require_fn_arity(params.len()),
            Expr::Application { lhs: _, args } => self.require_fn_arity(args.len()),
            Expr::Unit => self.require(Extension::UnitType),
            Expr::DotTuple { .. } | Expr::Tuple(_) => todo!("Is this `pairs` or `tuples`?"),
            Expr::Record(_) | Expr::DotRecord { .. } => self.require(Extension::Records),
            &Expr::Let {
                rec,
                bindings: _,
                body: _,
            } => self.require(if rec {
                Extension::LetrecBindings
            } else {
                Extension::LetBindings
            }),
            Expr::Ascription { .. } => self.require(Extension::TypeAscriptions),
            Expr::Inl(_) | Expr::Inr(_) | Expr::Match { .. } => self.require(Extension::SumTypes),
            Expr::List(_)
            | Expr::Cons(_)
            | Expr::ListHead(_)
            | Expr::ListIsEmpty(_)
            | Expr::ListTail(_) => self.require(Extension::Lists),
            Expr::Variant { lhs: _, rhs } => {
                self.require(Extension::Variants);
                if rhs.is_none() {
                    self.require(Extension::NullaryVariantLabels)
                }
            }
            Expr::Fix(_) => self.require(Extension::FixpointCombinators),

            Expr::Seq(_) => self.require(Extension::Sequencing),
            Expr::Assign { .. } | Expr::New(_) | Expr::Deref(_) | Expr::Address(_) => {
                self.require(Extension::References)
            }
            Expr::Panic => self.require(Extension::Panic),
            Expr::Throw(_) => self.require(Extension::Exceptions),
            Expr::TryWith { .. } | Expr::TryCatch { .. } => {
                todo!("#exceptions or #exception-type-declaration")
            }
            Expr::Cast { .. } => self.require(Extension::TypeCast),
            Expr::TryCastAs { .. } => self.require(Extension::TryCastAs),

            Expr::Generic { generics, expr } => todo!(),
            Expr::BinOp { lhs, op, rhs } => todo!(),
            Expr::TypeApplication { lhs, types } => todo!(),
            Expr::Not(expr) => todo!(),
            Expr::NatPred(expr) => todo!(),
            Expr::Fold { ty, expr } => todo!(),
            Expr::Unfold { ty, expr } => todo!(),
        }
        expr.visit_inner_with(self)
    }
}

pub(super) fn missing_extensions(program: &Program<'_>) -> Extensions {
    let mut collector = GateCollector::new();
    collector.visit_program(program);
    let GateCollector {
        mut required,
        is_in_global_scope: _,
    } = collector;
    for extension in &program.extensions {
        let _: bool = required.remove(extension);
    }
    required
}
