use std::collections::btree_set::BTreeSet;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, strum::IntoStaticStr, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum Extension {
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
    NaturalLiterals,
    NestedFunctionDeclarations,
    NullaryFunctions,
    MultiparameterFunctions,
    StructuralPatterns,
    NullaryVariantLabels,
    LetrecBindings,
    PatternAscriptions,
    LetPatterns,
}

pub(crate) type Extensions = BTreeSet<Extension>;
