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
    #[expect(clippy::match_same_arms, reason = "preserving order of variants")]
    pub(crate) const fn supported(&self) -> bool {
        match self {
            // Stage 1
            Self::UnitType
            | Self::Pairs
            | Self::Tuples
            | Self::Records
            | Self::LetBindings
            | Self::TypeAscriptions
            | Self::SumTypes
            | Self::Lists
            | Self::Variants
            | Self::FixpointCombinator => true,

            // Stage 1 (optional)
            Self::NaturalLiterals
            | Self::NestedFunctionDeclarations
            | Self::NullaryFunctions
            | Self::MultiparameterFunctions
            | Self::NullaryVariantLabels => true,
            Self::StructuralPatterns => false,
            Self::LetrecBindings => false,
            Self::PatternAscriptions => false,

            // Stage 1 (unclear if optional or mandatory)
            Self::LetPatterns => false,

            // Stage 2
            Self::Sequencing | Self::References | Self::Panic => true,
            Self::Exceptions => false,
            Self::ExceptionTypeDeclaration => false,
            Self::StructuralSubtyping => false,
            Self::TypeCast => false,
            Self::TopType | Self::BottomType | Self::AmbiguousTypeAsBottom => true,

            // Stage 2 (optional)
            Self::OpenVariantExceptions => false,
            Self::TryCastAs => false,
            Self::TypeCastPatterns => false,

            // Stage 3
            Self::TypeReconstruction => false,
            Self::UniversalTypes => false,

            // Never mentioned
            Self::ArithmeticOperators => false,
        }
    }
}
