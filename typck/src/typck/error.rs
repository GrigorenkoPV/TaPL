use core::fmt;
use std::{collections::btree_set::BTreeSet, process::ExitCode};

use crate::{
    ast::Decl,
    extensions::{Extension, Extensions},
};

#[expect(dead_code, reason = "WIP")]
#[derive(Debug, strum::IntoStaticStr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum Error<'src> {
    // Stage 1:
    //
    /// В программе отсутствует функция `main`.
    MissingMain,
    /// В выражении содержится необъявленная переменная.
    UndefinedVariable,
    /// Тип выражения отличается от ожидаемого.
    ///
    /// Эта ошибка должна возникать только если ни одна из более точных ошибок (ниже)
    /// не возникла раньше.
    UnexpectedTypeForExpression,
    /// При попытке применить (`Application`) выражение к аргументу
    /// или передать в комбинатор неподвижной точки (`Fix`), выражение
    /// оказывается не функцией.
    ///
    /// Ошибка должна возникать до проверки типа аргумента.
    NotAFunction,
    /// При попытке извлечь компонент кортежа (`DotTuple`) из выражения,
    /// выражение оказывается не кортежем (`TypeTuple`)
    NotATuple,
    /// При попытке извлечь поле записи (`DotRecord`) из выражения,
    /// выражение оказывается не записью (`TypeRecord`)
    NotARecord,
    /// При попытке извлечь голову (Head), хвост (Tail) или проверить список (`IsEmpty`),
    /// аргумент оказывается не списком (`TypeList`)
    NotAList,
    /// Анонимная функция (`Abstraction`) проверяется с не функциональным типом (`TypeFun`).
    ///
    /// Ошибка должна возникать до проверки типа самой функции.
    UnexpectedLambda,
    /// Тип параметра анонимной функции (`AParamDecl`) отличается от ожидаемого.
    ///
    /// Ошибка должна возникать до проверки тела функции.
    UnexpectedTypeForParameter,
    /// Кортеж (`Tuple`) проверяется с типом, отличным от типа кортежа (`TypeTuple`).
    ///
    /// Ошибка до проверки элементов.
    UnexpectedTuple,
    /// Запись (`Record`) проверяется с типом, отличным от типа записи (`TypeRecord`).
    ///
    /// Ошибка до проверки полей.
    UnexpectedRecord,
    /// Вариант (`Variant`) проверяется с типом, отличным от типа варианта (`TypeVariant`).
    ///
    /// Ошибка до проверки варианта
    UnexpectedVariant,
    /// Список (`List` или `ConsList`) проверяется с типом, отличным от типа списка (`TypeList`).
    ///
    /// Ошибка до проверки элементов
    UnexpectedList,
    /// Инъекция (`Inl` или `Inr`) проверяется с типом, отличным от типа-суммы (`TypeSum`).
    ///
    /// Ошибка до проверки инъекции.
    UnexpectedInjection,
    /// В записи (`Record`) отсутствуют ожидаемые поля.
    MissingRecordFields,
    /// В записи (`Record`) присутствуют поля, которых нет в ожидаемом типе записи.
    UnexpectedRecordFields,
    /// Попытка извлечь отсутствующее поле записи (`DotRecord`).
    UnexpectedFieldAccess,
    /// В варианте (Variant) используется тег, которого нет в ожидаемом типе варианта.
    UnexpectedVariantLabel,
    /// Попытка извлечь отсутствующий компонент кортежа (`DotTuple`).
    TupleIndexOutOfBounds,
    /// Длина кортежа (`Tuple`) не соответствует ожидаемой длине.
    UnexpectedTupleLength,
    /// Тип инъекции (`Inl` или `Inr`) невозможно определить (отсутствует ожидаемый тип-сумма).
    AmbiguousSumType,
    /// Тип варианта (`Variant`) невозможно определить (отсутствует ожидаемый тип варианта).
    AmbiguousVariantType,
    /// Тип списка (`List` или `ConsList`) невозможно определить (отсутствует ожидаемый тип списка).
    AmbiguousList,
    /// `match`-выражение с пустым списком альтернатив.
    IllegalEmptyMatching,
    /// Не все образцы перечислены в `match`-выражении
    /// (`inl` и `inr` для типа-суммы, все теги для варианта).
    NonexhaustiveMatchPatterns,
    /// Образец в match-выражении не соответствует типу разбираемого выражения.
    UnexpectedPatternForType,
    /// В записи (`Record`) указаны повторяющиеся имена полей.
    DuplicateRecordFields,
    /// В типе записи (`TypeRecord`) указаны повторяющиеся имена полей.
    DuplicateRecordTypeFields,
    /// В типе варианта (`TypeVariant`) указаны повторяющиеся имена вариантов (тегов/полей).
    DuplicateVariantTypeFields,
    /// Функция с данным именем объявлена в программе более одного раза.
    DuplicateFunctionDeclaration([&'src Decl<'src>; 2]),

    // Stage 1 extra 3:
    //
    /// Функция main объявлена с `n` параметрами, где `n != 1`.
    IncorrectArityOfMain,
    /// Вызов функции (`Application`) происходит с некорректным количеством аргументов.
    IncorrectNumberOfArguments,
    /// Количество параметров анонимной функции (`Abstraction`)
    /// не совпадает с ожидаемым количеством параметров.
    UnexpectedNumberOfParametersInLambda,

    // Stage 1 extra 4:
    //
    /// В паттерне записи (`PatternRecord`) указаны повторяющиеся имена полей.
    DuplicateRecordPatternFields,

    // Stage 1 extra 5:
    //
    /// Вариант (`Variant`) содержит данные (`SomeExprData`),
    /// хотя ожидается тег без данных (`NoTyping`).
    UnexpectedDataForNullaryLabel,
    /// Вариант (`Variant`) не содержит данные (`NoExprData`),
    /// хотя ожидается тег с данными (`SomeTyping`).
    MissingDataForLabel,
    /// Образец варианта (`PatternVariant`) содержит тег с данными (`SomePatternData`),
    /// хотя в типе разбираемого выражения этот тег указан без данных (`NoTyping`).
    UnexpectedNonNullaryVariantPattern,
    /// Образец варианта (`PatternVariant`) содержит тег без данных (`NoPatternData`),
    /// хотя в типе разбираемого выражения этот тег указан с данными (`SomeTyping`).
    UnexpectedNullaryVariantPattern,

    // Stage 1 extra 7:
    //
    /// В объявлении функции повторяется имя параметра.
    DuplicateFunctionParameter,
    /// Повторяющееся связывание в `let`-выражении.
    DuplicateLetBinding,
    /// Повторяющийся параметр типа.
    DuplicateTypeParameter,

    // Stage 2:
    //
    /// В программе используются исключения, но не объявлен их тип.
    ExceptionTypeNotDeclared,
    /// Неоднозначный тип `throw`-выражения (`Throw`).
    AmbiguousThrowType,
    /// Неоднозначный тип адреса памяти (`ConstMemory`).
    AmbiguousReferenceType,
    /// Неоднозначный тип ошибки (`Panic`).
    AmbiguousPanicType,
    /// Попытка разыменовать (`Deref`) или присвоить значение (`Assign`)
    /// выражению не ссылочного типа.
    NotAReference,
    /// Адрес памяти (`ConstMemory`) используется там,
    /// где ожидается тип, отличный от типа-ссылки (`TypeRef`).
    UnexpectedMemoryAddress,
    /// Выражение создания ссылки (`Ref`, т. е. `new(<выражение>)`)
    /// используется там, где ожидается не ссылочный тип.
    UnexpectedReference,
    /// Тип выражения не является подтипом ожидаемого.
    ///
    /// Эта ошибка должна возникать только если ни одна из более специфичных ошибок
    /// (перечисленных выше вместе с кодами Этапа 1) не возникла раньше.
    UnexpectedSubtype,

    // Stage 3:
    //
    /// Во время унификации возникает ограничение, порождающее (запрещённый) бесконечный тип.
    OccursCheckInfiniteType,
    /// При попытке применить (`TypeApplication`) универсальное выражение к типовому аргументу,
    /// выражение оказывается не универсальной функцией.
    ///
    /// Ошибка должна возникать до проверки аргумента.
    NotAGenericFunction,
    /// Вызов универсальной функции (`TypeApplication`)
    /// происходит с некорректным количеством типов-аргументов.
    IncorrectNumberOfTypeArguments,
    /// В типе содержится необъявленная типовая переменная (только для `#universal-types`)
    UndefinedTypeVariable,

    // Custom:
    //
    UnsupportedExtensions(BTreeSet<&'src Extension>),
    MissingExtensions(Extensions),
}

impl core::error::Error for Error<'_> {}

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ERROR_{}", <&'static str>::from(self))?;

        #[expect(clippy::wildcard_enum_match_arm, reason = "WIP")]
        match self {
            Self::MissingMain => write!(f, "`main` is missing or is not a function"),

            _ => todo!(),
        }
    }
}

impl From<Error<'_>> for ExitCode {
    fn from(err: Error<'_>) -> ExitCode {
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
            Error::NonexhaustiveMatchPatterns => 25,
            Error::UnexpectedPatternForType => 26,
            Error::DuplicateRecordFields => 27,
            Error::DuplicateRecordTypeFields => 28,
            Error::DuplicateVariantTypeFields => 29,
            Error::DuplicateFunctionDeclaration(_) => 30,

            Error::IncorrectArityOfMain
            | Error::IncorrectNumberOfArguments
            | Error::UnexpectedNumberOfParametersInLambda => 113,
            Error::DuplicateRecordPatternFields => 114,
            Error::UnexpectedDataForNullaryLabel
            | Error::MissingDataForLabel
            | Error::UnexpectedNonNullaryVariantPattern
            | Error::UnexpectedNullaryVariantPattern => 115,
            Error::DuplicateFunctionParameter
            | Error::DuplicateLetBinding
            | Error::DuplicateTypeParameter => 117,

            Error::ExceptionTypeNotDeclared => 31,
            Error::AmbiguousThrowType => 32,
            Error::AmbiguousReferenceType => 33,
            Error::AmbiguousPanicType => 34,
            Error::NotAReference => 35,
            Error::UnexpectedMemoryAddress => 36,
            Error::UnexpectedReference => 37,
            Error::UnexpectedSubtype => 38,

            Error::OccursCheckInfiniteType => 39,
            Error::NotAGenericFunction => 40,
            Error::IncorrectNumberOfTypeArguments => 41,
            Error::UndefinedTypeVariable => 42,

            Error::UnsupportedExtensions(_) => 253,
            Error::MissingExtensions(_) => 254,
        }
        .into()
    }
}
