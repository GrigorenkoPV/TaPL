use core::{error, fmt};
use std::process::ExitCode;

use culpa::throws;

use crate::ast::Program;

#[derive(Debug)]
pub(crate) enum Error {
    MissingMain,
    UndefinedVariable,
    UnexpectedTypeForExpression,
    NotAFunction,
    NotATuple,
    NotARecord,
    NotAList,
    UnexpectedLambda,
    UnexpectedTypeForParameter,
    UnexpectedTuple,
    UnexpectedRecord,
    UnexpectedVariant,
    UnexpectedList,
    UnexpectedInjection,
    MissingRecordFields,
    UnexpectedRecordFields,
    UnexpectedFieldAccess,
    UnexpectedVariantLabel,
    TupleIndexOutOfBounds,
    UnexpectedTupleLength,
    AmbiguousSumType,
    AmbiguousVariantType,
    AmbiguousList,
    IllegalEmptyMatching,
    NonExhaustiveMatchPatterns,
    UnexpectedPatternForType,
    DuplicateRecordFields,
    DuplicateRecordTypeFields,
    DuplicateVariantTypeFields,
    DuplicateFunctionDeclaration,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMain => todo!(),
            Self::UndefinedVariable => todo!(),
            Self::UnexpectedTypeForExpression => todo!(),
            Self::NotAFunction => todo!(),
            Self::NotATuple => todo!(),
            Self::NotARecord => todo!(),
            Self::NotAList => todo!(),
            Self::UnexpectedLambda => todo!(),
            Self::UnexpectedTypeForParameter => todo!(),
            Self::UnexpectedTuple => todo!(),
            Self::UnexpectedRecord => todo!(),
            Self::UnexpectedVariant => todo!(),
            Self::UnexpectedList => todo!(),
            Self::UnexpectedInjection => todo!(),
            Self::MissingRecordFields => todo!(),
            Self::UnexpectedRecordFields => todo!(),
            Self::UnexpectedFieldAccess => todo!(),
            Self::UnexpectedVariantLabel => todo!(),
            Self::TupleIndexOutOfBounds => todo!(),
            Self::UnexpectedTupleLength => todo!(),
            Self::AmbiguousSumType => todo!(),
            Self::AmbiguousVariantType => todo!(),
            Self::AmbiguousList => todo!(),
            Self::IllegalEmptyMatching => todo!(),
            Self::NonExhaustiveMatchPatterns => todo!(),
            Self::UnexpectedPatternForType => todo!(),
            Self::DuplicateRecordFields => todo!(),
            Self::DuplicateRecordTypeFields => todo!(),
            Self::DuplicateVariantTypeFields => todo!(),
            Self::DuplicateFunctionDeclaration => todo!(),
        }
    }
}

impl From<Error> for ExitCode {
    fn from(err: Error) -> ExitCode {
        match err {
            Error::MissingMain => 1,
            Error::UndefinedVariable => 2,
            Error::UnexpectedTypeForExpression => 3,
            Error::NotAFunction => 4,
            Error::NotATuple => 5,
            Error::NotARecord => 6,
            Error::NotAList => 7,
            Error::UnexpectedLambda => 8,
            Error::UnexpectedTypeForParameter => 9,
            Error::UnexpectedTuple => 10,
            Error::UnexpectedRecord => 11,
            Error::UnexpectedVariant => 12,
            Error::UnexpectedList => 13,
            Error::UnexpectedInjection => 14,
            Error::MissingRecordFields => 15,
            Error::UnexpectedRecordFields => 16,
            Error::UnexpectedFieldAccess => 17,
            Error::UnexpectedVariantLabel => 18,
            Error::TupleIndexOutOfBounds => 19,
            Error::UnexpectedTupleLength => 20,
            Error::AmbiguousSumType => 21,
            Error::AmbiguousVariantType => 22,
            Error::AmbiguousList => 23,
            Error::IllegalEmptyMatching => 24,
            Error::NonExhaustiveMatchPatterns => 25,
            Error::UnexpectedPatternForType => 26,
            Error::DuplicateRecordFields => 27,
            Error::DuplicateRecordTypeFields => 28,
            Error::DuplicateVariantTypeFields => 29,
            Error::DuplicateFunctionDeclaration => 30,
        }
        .into()
    }
}

#[throws]
pub(crate) fn typck(program: &Program<'_>) {
    todo!()
}
