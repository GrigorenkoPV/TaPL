use core::fmt::{self, Write};
use std::collections::btree_set::BTreeSet;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, strum::IntoStaticStr, strum::EnumString,
)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum Extension {
    // Stage 1
    UnitType,
    Pairs,
    Tuples,
    Records,
    LetBindings,
    TypeAscriptions,
    SumTypes,
    Lists,
    Variants,
    FixpointCombinator,

    // Stage 1 (optional)
    NaturalLiterals,
    NestedFunctionDeclarations,
    NullaryFunctions,
    MultiparameterFunctions,
    StructuralPatterns,
    NullaryVariantLabels,
    LetrecBindings,
    PatternAscriptions,

    // Stage 1 (unclear if optional or mandatory)
    LetPatterns,

    // Stage 2
    Sequencing,
    References,
    Panic,
    Exceptions,
    ExceptionTypeDeclaration,
    StructuralSubtyping,
    TypeCast,
    TopType,
    BottomType,
    AmbiguousTypeAsBottom,

    // Stage 2 (optional)
    OpenVariantExceptions,
    TryCastAs,
    TypeCastPatterns,

    // Stage 3
    TypeReconstruction,
    UniversalTypes,

    // Never mentioned
    ArithmeticOperators,
}

pub(crate) type Extensions = BTreeSet<Extension>;

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('#')?;
        f.write_str(self.into())
    }
}

impl Extension {
    #[must_use]
    pub(crate) const fn supported(&self) -> bool {
        match self {
            Self::NullaryFunctions
            | Self::MultiparameterFunctions
            | Self::NaturalLiterals
            | Self::NestedFunctionDeclarations
            | Self::LetBindings
            | Self::Panic
            | Self::UnitType => true,

            Self::Pairs
            | Self::Tuples
            | Self::Records
            | Self::TypeAscriptions
            | Self::SumTypes
            | Self::Lists
            | Self::Variants
            | Self::FixpointCombinator
            | Self::StructuralPatterns
            | Self::NullaryVariantLabels
            | Self::LetrecBindings
            | Self::PatternAscriptions
            | Self::LetPatterns
            | Self::Sequencing
            | Self::References
            | Self::Exceptions
            | Self::ExceptionTypeDeclaration
            | Self::StructuralSubtyping
            | Self::TypeCast
            | Self::TopType
            | Self::BottomType
            | Self::AmbiguousTypeAsBottom
            | Self::OpenVariantExceptions
            | Self::TryCastAs
            | Self::TypeCastPatterns
            | Self::TypeReconstruction
            | Self::ArithmeticOperators
            | Self::UniversalTypes => false,
        }
    }
}

#[expect(clippy::tests_outside_test_module, reason = "just a single test")]
#[test]
fn supported() {
    if Extension::Tuples.supported() {
        assert!(
            Extension::Pairs.supported(),
            "tuples are a superset of pairs"
        )
    }
}

pub(crate) mod unsupported {
    use crate::ast::{Annotation, Expr, Function, Type};
    use core::debug_assert_matches;

    #[track_caller]
    pub(crate) fn mu(ty: &Type<'_>) -> ! {
        debug_assert_matches!(ty, Type::Rec { .. }, "This is not a recursive type");
        unimplemented!("rec types are not supported")
    }

    #[track_caller]
    pub(crate) fn fold_unfold(expr: &Expr<'_>) -> ! {
        debug_assert_matches!(
            expr,
            Expr::Fold { .. } | Expr::Unfold { .. },
            "This is not fold/unfold"
        );
        unimplemented!("fold/unfold are not supported")
    }

    #[track_caller]
    pub(crate) fn annotations(_: &Annotation) -> ! {
        unimplemented!("functions with annotations are not supported")
    }

    #[track_caller]
    pub(crate) fn throws(function: &Function<'_>) -> ! {
        debug_assert!(!function.throws.is_empty(), "This function does not throw");
        unimplemented!("functions that throw are not supported")
    }

    #[track_caller]
    pub(crate) fn no_ret_ty(function: &Function<'_>) -> ! {
        debug_assert_matches!(
            function.ret_ty,
            None,
            "This function does have a return type"
        );
        unimplemented!("functions without an explicit return type are not supported")
    }
}
