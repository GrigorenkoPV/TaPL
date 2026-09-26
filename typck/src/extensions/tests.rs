use strum::VariantArray;

use crate::extensions::Extensions;

use super::Extension;

#[test]
fn subextensions() {
    if Extension::Tuples.supported() {
        assert!(
            Extension::Pairs.supported(),
            "tuples are a superset of pairs"
        )
    }
    if Extension::NullaryVariantLabels.supported() {
        assert!(
            Extension::Variants.supported(),
            "it's nullary VARIANT labels"
        )
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum Stage {
    Stage1,
    Stage2,
    Stage3,
}

impl Extension {
    #[must_use]
    const fn required_by(&self) -> Option<Stage> {
        match self {
            Self::UnitType
            | Self::Pairs
            | Self::Tuples
            | Self::Records
            | Self::LetBindings
            | Self::TypeAscriptions
            | Self::SumTypes
            | Self::Lists
            | Self::Variants
            | Self::FixpointCombinator => Some(Stage::Stage1),

            Self::NaturalLiterals
            | Self::NestedFunctionDeclarations
            | Self::NullaryFunctions
            | Self::MultiparameterFunctions
            | Self::StructuralPatterns
            | Self::NullaryVariantLabels
            | Self::LetrecBindings
            | Self::PatternAscriptions => {
                // stage 1 optional
                None
            }

            Self::LetPatterns => {
                // stage 1, not clear if mandatory or optional
                None
            }

            Self::Sequencing
            | Self::References
            | Self::Panic
            | Self::Exceptions
            | Self::ExceptionTypeDeclaration
            | Self::StructuralSubtyping
            | Self::TypeCast
            | Self::TopType
            | Self::BottomType
            | Self::AmbiguousTypeAsBottom => Some(Stage::Stage2),

            Self::OpenVariantExceptions | Self::TryCastAs | Self::TypeCastPatterns => {
                // stage 2 optional
                None
            }

            Self::TypeReconstruction | Self::UniversalTypes => Some(Stage::Stage3),

            Self::ArithmeticOperators => {
                // never mentioned
                None
            }
        }
    }
}

#[track_caller]
fn check_everything_supported(stage: Stage) {
    let mut unsupported = Extensions::new();
    for ext in Extension::VARIANTS {
        if ext.required_by() == Some(stage) && !ext.supported() {
            let _: bool = unsupported.insert(ext.clone());
        }
    }
    assert!(
        unsupported.is_empty(),
        "Some {stage:?} extensions are unsupported: {unsupported:?}"
    )
}

#[test]
fn stage1() {
    check_everything_supported(Stage::Stage1);
}

#[test]
#[should_panic(
    expected = "Some Stage2 extensions are unsupported: {Exceptions, ExceptionTypeDeclaration, StructuralSubtyping, TypeCast, TopType, BottomType}"
)]
fn stage2() {
    check_everything_supported(Stage::Stage2);
}

#[test]
#[should_panic(
    expected = "Some Stage3 extensions are unsupported: {TypeReconstruction, UniversalTypes}"
)]
fn stage3() {
    check_everything_supported(Stage::Stage3);
}
