use std::collections::btree_set::BTreeSet;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, strum::IntoStaticStr, strum::EnumString)]
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
    FixpointCombinators,

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
}

pub(crate) type Extensions = BTreeSet<Extension>;
