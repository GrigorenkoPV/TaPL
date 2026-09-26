use core::fmt::{self, Write};
use std::collections::btree_set::BTreeSet;

#[cfg(test)]
mod tests;

pub(crate) mod unsupported;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::IntoStaticStr,
    strum::EnumString,
    strum::VariantArray,
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
            | Self::TypeAscriptions
            | Self::SumTypes
            | Self::Lists
            | Self::FixpointCombinator
            | Self::Pairs
            | Self::Tuples
            | Self::Records
            | Self::Variants
            | Self::UnitType => true,

            Self::StructuralPatterns
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
