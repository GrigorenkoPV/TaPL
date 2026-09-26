use core::mem;
use std::collections::btree_set::BTreeSet;

use culpa::{throw, throws};

use crate::{
    ast::{
        Annotation, Binding, Decl, Expr, Function, HasVisitable, Pattern, PatternBinding, Program,
        Type, Visitor,
    },
    extensions::{Extension, Extensions, unsupported},
    typck::Error,
};

#[derive(Debug)]
struct GateCollector<'a> {
    enabled: &'a Extensions,
    missing: Extensions,
    missing_one_of: BTreeSet<Extensions>,
    is_in_global_scope: bool,
}

impl GateCollector<'_> {
    #[expect(
        clippy::collapsible_if,
        clippy::needless_pass_by_value,
        reason = "`insert` does not return the reference to the inserted element
        BTreeSet entries are unstable
        so a needless `clone` it is instead
        "
    )]
    fn require(&mut self, extension: Extension) {
        if !self.enabled.contains(&extension) {
            if self.missing.insert(extension.clone()) {
                self.missing_one_of
                    .retain(|options| !options.contains(&extension));
            }
        }
    }
    fn require_one_of(&mut self, extensions: Extensions) {
        debug_assert!(!extensions.is_empty(), "Cannot require one of ∅");
        for ext in &extensions {
            if self.enabled.contains(ext) {
                return;
            }
        }
        for ext in &self.missing {
            if extensions.contains(ext) {
                return;
            }
        }
        for other in &self.missing_one_of {
            if extensions.is_subset(other) {
                return;
            }
        }
        let inserted: bool = self.missing_one_of.insert(extensions);
        debug_assert!(inserted, "should've bailed on `is_subset` otherwise");
    }

    fn require_fn_arity(&mut self, arity: usize) {
        match arity {
            0 => self.require(Extension::NullaryFunctions),
            1 => {}
            _ => self.require(Extension::MultiparameterFunctions),
        }
    }

    fn require_pairs_or_tuples(&mut self) {
        self.require_one_of([Extension::Pairs, Extension::Tuples].into());
    }

    fn require_tuple_arity(&mut self, arity: usize) {
        if arity == 2 {
            self.require_pairs_or_tuples()
        } else {
            self.require(Extension::Tuples)
        }
    }
}

impl<'ctx> Visitor<'ctx> for GateCollector<'_> {
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
            self.require(Extension::UniversalTypes)
        }
        self.require_fn_arity(params.len());
        if ret_ty.is_none() {
            unsupported::no_ret_ty(function)
        }
        if !throws.is_empty() {
            unsupported::throws(function)
        }
        let was_in_global_scope = mem::replace(&mut self.is_in_global_scope, false);
        function.visit_inner_with(self);
        self.is_in_global_scope = was_in_global_scope;
    }

    fn visit_annotation(&mut self, annotation: &Annotation) {
        unsupported::annotations(annotation)
    }

    fn visit_pattern_binding(&mut self, pattern_binding: &PatternBinding<'ctx>) {
        let PatternBinding { lhs, rhs: _ } = pattern_binding;
        if !matches!(lhs, Pattern::Var(_)) {
            self.require(Extension::LetPatterns)
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

            Decl::ExceptionType(_) => self.require(Extension::ExceptionTypeDeclaration),
            Decl::ExceptionVariant { .. } => self.require(Extension::OpenVariantExceptions),

            Decl::TypeAlias { .. } => unimplemented!("Type aliases are not supported"),
        }
        decl.visit_inner_with(self)
    }

    fn visit_ty(&mut self, ty: &Type<'ctx>) {
        match ty {
            Type::Bool | Type::Nat => {}
            Type::Fn { args, ret: _ } => self.require_fn_arity(args.len()),
            Type::Unit => self.require(Extension::UnitType),
            Type::Tuple(inner) => self.require_tuple_arity(inner.len()),
            Type::Record(_) => self.require(Extension::Records),
            Type::Sum { .. } => self.require(Extension::SumTypes),
            Type::List(_) => self.require(Extension::Lists),
            Type::Variant(variants) => {
                self.require(Extension::Variants);
                for (_, ty) in variants {
                    if ty.is_none() {
                        self.require(Extension::NullaryVariantLabels)
                    }
                }
            }

            Type::Ref(_) => self.require(Extension::References),
            Type::Top => self.require(Extension::TopType),
            Type::Bot => self.require(Extension::BottomType),

            Type::Auto => self.require(Extension::TypeReconstruction),
            Type::Var(_) | Type::Forall { .. } => self.require(Extension::UniversalTypes),

            t @ Type::Rec { .. } => unsupported::mu(t),
        }
        ty.visit_inner_with(self)
    }

    fn visit_pat(&mut self, pat: &Pattern<'ctx>) {
        match pat {
            Pattern::Var(_) => {
                self.require_one_of(
                    [
                        Extension::LetBindings,
                        Extension::SumTypes,
                        Extension::Variants,
                        Extension::StructuralPatterns,
                    ]
                    .into(),
                );
            }
            Pattern::Inl(_) | Pattern::Inr(_) => self.require(Extension::SumTypes),
            Pattern::Variant { label: _, pat } => {
                self.require(Extension::Variants);
                if pat.is_none() {
                    self.require(Extension::NullaryVariantLabels)
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

    fn visit_expr(&mut self, expr: &Expr<'ctx>) {
        match expr {
            Expr::True
            | Expr::False
            | Expr::If { .. }
            | Expr::Succ(_)
            | Expr::NatPred(_)
            | Expr::NatIsZero(_)
            | Expr::NatRec { .. }
            | Expr::Var(_)
            | Expr::Match { .. } => { /* subexpressions are visited below */ }

            &Expr::Integer(i) => {
                if i != 0 {
                    self.require(Extension::NaturalLiterals)
                }
            }
            Expr::Abstraction { params, ret: _ } => self.require_fn_arity(params.len()),
            Expr::Application { lhs: _, args } => self.require_fn_arity(args.len()),
            Expr::Unit => self.require(Extension::UnitType),
            Expr::DotTuple { .. } => self.require_pairs_or_tuples(),
            Expr::Tuple(inner) => self.require_tuple_arity(inner.len()),
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
            Expr::Inl(_) | Expr::Inr(_) => self.require(Extension::SumTypes),

            Expr::List(_)
            | Expr::Cons { .. }
            | Expr::ListHead(_)
            | Expr::ListIsEmpty(_)
            | Expr::ListTail(_) => self.require(Extension::Lists),
            Expr::Variant { label: _, data } => {
                self.require(Extension::Variants);
                if data.is_none() {
                    self.require(Extension::NullaryVariantLabels)
                }
            }
            Expr::Fix(_) => self.require(Extension::FixpointCombinator),

            Expr::Seq(_) => self.require(Extension::Sequencing),
            Expr::Assign { .. } | Expr::New(_) | Expr::Deref(_) | Expr::Address(_) => {
                self.require(Extension::References)
            }
            Expr::Panic => self.require(Extension::Panic),
            Expr::Throw(_) | Expr::TryWith { .. } | Expr::TryCatch { .. } => {
                self.require(Extension::Exceptions)
            }
            Expr::Cast { .. } => self.require(Extension::TypeCast),
            Expr::TryCastAs { .. } => self.require(Extension::TryCastAs),

            Expr::Generic { .. } | Expr::TypeApplication { .. } => {
                self.require(Extension::UniversalTypes)
            }

            Expr::BinOp { .. } | Expr::Not(..) => self.require(Extension::ArithmeticOperators),
            e @ (Expr::Fold { .. } | Expr::Unfold { .. }) => unsupported::fold_unfold(e),
        }
        expr.visit_inner_with(self)
    }
}

#[throws(Error<'src>)]
fn check_unsupported<'src>(program: &'src Program<'src>) {
    let mut unsupported = BTreeSet::<&'src Extension>::new();
    for ext in &program.extensions {
        if !ext.supported() {
            let inserted = unsupported.insert(ext);
            debug_assert!(inserted, "We're iterating over a set")
        }
    }
    if !unsupported.is_empty() {
        throw!(Error::UnsupportedExtensions(unsupported))
    }
}

#[throws(Error<'src>)]
fn check_missing<'src>(program: &'src Program<'src>) {
    let mut collector = GateCollector {
        enabled: &program.extensions,
        missing: Extensions::new(),
        missing_one_of: BTreeSet::new(),
        is_in_global_scope: true,
    };
    collector.visit_program(program);
    let GateCollector {
        enabled,
        missing,
        missing_one_of,
        is_in_global_scope: _,
    } = collector;
    if !missing.is_empty() || !missing_one_of.is_empty() {
        for ext in missing.iter().chain(missing_one_of.iter().flatten()) {
            debug_assert!(
                !enabled.contains(ext),
                "\"Missing\" extension {ext:?} is actually enabled"
            )
        }
        throw!(Error::MissingExtensions {
            mandatory: missing,
            choosable: missing_one_of
        })
    }
}

#[throws(Error<'src>)]
pub(super) fn check_extensions<'src>(program: &'src Program<'src>) {
    check_missing(program)?;
    check_unsupported(program)?;
}
