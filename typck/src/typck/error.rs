use core::{debug_assert_matches, fmt, mem, num::NonZero};
use std::{borrow::Cow, collections::btree_set::BTreeSet};

use crate::{
    ast::{Binding, Expr, Ident, Integer, Type},
    extensions::{Extension, Extensions},
};

#[derive(Debug)]
pub(crate) enum RecordWithMissingFields<'src> {
    Literal(&'src Expr<'src>),
    Subtyping {
        actual_expr: Option<&'src Expr<'src>>,
        actual_type: Type<'src>,
    },
}

#[derive(Debug, strum::IntoStaticStr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum Error<'src> {
    // Stage 1:
    //
    /// В программе отсутствует функция `main`.
    #[expect(clippy::empty_enum_variants_with_brackets, reason = "intentional")]
    MissingMain {},
    /// В выражении содержится необъявленная переменная.
    UndefinedVariable(Ident<'src>),
    /// Тип выражения отличается от ожидаемого.
    ///
    /// Эта ошибка должна возникать только если ни одна из более точных ошибок (ниже)
    /// не возникла раньше.
    UnexpectedTypeForExpression {
        expr: &'src Expr<'src>,
        actual: Type<'src>,
        expected: Type<'src>,
    },
    /// При попытке применить (`Application`) выражение к аргументу
    /// или передать в комбинатор неподвижной точки (`Fix`), выражение
    /// оказывается не функцией.
    ///
    /// Ошибка должна возникать до проверки типа аргумента.
    NotAFunction {
        expr: &'src Expr<'src>,
        actual: Type<'src>,
    },
    /// При попытке извлечь компонент кортежа (`DotTuple`) из выражения,
    /// выражение оказывается не кортежем (`TypeTuple`)
    NotATuple {
        expr: &'src Expr<'src>,
        actual: Type<'src>,
    },
    /// При попытке извлечь поле записи (`DotRecord`) из выражения,
    /// выражение оказывается не записью (`TypeRecord`)
    NotARecord {
        expr: &'src Expr<'src>,
        actual: Type<'src>,
    },
    /// При попытке извлечь голову (Head), хвост (Tail) или проверить список (`IsEmpty`),
    /// аргумент оказывается не списком (`TypeList`)
    NotAList {
        expr: &'src Expr<'src>,
        actual: Type<'src>,
    },
    /// Анонимная функция (`Abstraction`) проверяется с не функциональным типом (`TypeFun`).
    ///
    /// Ошибка должна возникать до проверки типа самой функции.
    UnexpectedLambda {
        lambda: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Тип параметра анонимной функции (`AParamDecl`) отличается от ожидаемого.
    ///
    /// Ошибка должна возникать до проверки тела функции.
    UnexpectedTypeForParameter {
        param: Ident<'src>,
        actual: Type<'src>,
        expected: Type<'src>,
    },
    /// Кортеж (`Tuple`) проверяется с типом, отличным от типа кортежа (`TypeTuple`).
    ///
    /// Ошибка до проверки элементов.
    UnexpectedTuple {
        tuple: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Запись (`Record`) проверяется с типом, отличным от типа записи (`TypeRecord`).
    ///
    /// Ошибка до проверки полей.
    UnexpectedRecord {
        record: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Вариант (`Variant`) проверяется с типом, отличным от типа варианта (`TypeVariant`).
    ///
    /// Ошибка до проверки варианта
    UnexpectedVariant {
        variant: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Список (`List` или `ConsList`) проверяется с типом, отличным от типа списка (`TypeList`).
    ///
    /// Ошибка до проверки элементов
    UnexpectedList {
        list: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Инъекция (`Inl` или `Inr`) проверяется с типом, отличным от типа-суммы (`TypeSum`).
    ///
    /// Ошибка до проверки инъекции.
    UnexpectedInjection {
        injection: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// В записи (`Record`) отсутствуют ожидаемые поля.
    MissingRecordFields {
        actual: RecordWithMissingFields<'src>,
        expected: Type<'src>,
        missing: Ident<'src>,
    },
    /// В записи (`Record`) присутствуют поля, которых нет в ожидаемом типе записи.
    UnexpectedRecordFields {
        actual: &'src Expr<'src>,
        expected: Type<'src>,
        unexpected: Ident<'src>,
    },
    /// Попытка извлечь отсутствующее поле записи (`DotRecord`).
    UnexpectedFieldAccess {
        record: &'src Expr<'src>,
        expected: Ident<'src>,
        actual: Box<[(Ident<'src>, Type<'src>)]>,
    },
    /// В варианте (Variant) используется тег, которого нет в ожидаемом типе варианта.
    UnexpectedVariantLabel {
        expected: Type<'src>,
        actual: Ident<'src>,
    },
    /// Попытка извлечь отсутствующий компонент кортежа (`DotTuple`).
    TupleIndexOutOfBounds {
        tuple: &'src Expr<'src>,
        length: usize,
        index: Integer,
    },
    /// Длина кортежа (`Tuple`) не соответствует ожидаемой длине.
    UnexpectedTupleLength {
        tuple: &'src Expr<'src>,
        actual: usize,
        expected: usize,
    },
    /// Тип инъекции (`Inl` или `Inr`) невозможно определить (отсутствует ожидаемый тип-сумма).
    AmbiguousSumType(&'src Expr<'src>),
    /// Тип варианта (`Variant`) невозможно определить (отсутствует ожидаемый тип варианта).
    AmbiguousVariantType(&'src Expr<'src>),
    /// Тип списка (`List` или `ConsList`) невозможно определить (отсутствует ожидаемый тип списка).
    #[expect(clippy::empty_enum_variants_with_brackets, reason = "intentional")]
    AmbiguousList {},
    /// `match`-выражение с пустым списком альтернатив.
    #[expect(dead_code, reason = "WIP")]
    IllegalEmptyMatching,
    /// Не все образцы перечислены в `match`-выражении
    /// (`inl` и `inr` для типа-суммы, все теги для варианта).
    #[expect(dead_code, reason = "WIP")]
    NonexhaustiveMatchPatterns,
    /// Образец в match-выражении не соответствует типу разбираемого выражения.
    #[expect(dead_code, reason = "WIP")]
    UnexpectedPatternForType,
    /// В записи (`Record`) указаны повторяющиеся имена полей.
    DuplicateRecordFields {
        expr: &'src Expr<'src>,
        field: Ident<'src>,
    },
    /// В типе записи (`TypeRecord`) указаны повторяющиеся имена полей.
    DuplicateRecordTypeFields(Ident<'src>),
    /// В типе варианта (`TypeVariant`) указаны повторяющиеся имена вариантов (тегов/полей).
    DuplicateVariantTypeFields(Ident<'src>),
    /// Функция с данным именем объявлена в программе более одного раза.
    DuplicateFunctionDeclaration(Ident<'src>),

    // Stage 1 extra 3:
    //
    /// Функция main объявлена с `n` параметрами, где `n != 1`.
    IncorrectArityOfMain(usize),
    /// Вызов функции (`Application`) происходит с некорректным количеством аргументов.
    IncorrectNumberOfArguments {
        expected: usize,
        actual: usize,
        callee: &'src Expr<'src>,
    },
    /// Количество параметров анонимной функции (`Abstraction`)
    /// не совпадает с ожидаемым количеством параметров.
    UnexpectedNumberOfParametersInLambda {
        expected: usize,
        actual: usize,
        lambda: &'src Expr<'src>,
    },

    // Stage 1 extra 4:
    //
    /// В паттерне записи (`PatternRecord`) указаны повторяющиеся имена полей.
    #[expect(dead_code, reason = "WIP")]
    DuplicateRecordPatternFields(Ident<'src>),

    // Stage 1 extra 5:
    //
    /// Вариант (`Variant`) содержит данные (`SomeExprData`),
    /// хотя ожидается тег без данных (`NoTyping`).
    UnexpectedDataForNullaryLabel {
        label: Ident<'src>,
        unexpected: &'src Expr<'src>,
    },
    /// Вариант (`Variant`) не содержит данные (`NoExprData`),
    /// хотя ожидается тег с данными (`SomeTyping`).
    MissingDataForLabel {
        label: Ident<'src>,
        expected: Type<'src>,
    },
    /// Образец варианта (`PatternVariant`) содержит тег с данными (`SomePatternData`),
    /// хотя в типе разбираемого выражения этот тег указан без данных (`NoTyping`).
    #[expect(dead_code, reason = "WIP")]
    UnexpectedNonNullaryVariantPattern,
    /// Образец варианта (`PatternVariant`) содержит тег без данных (`NoPatternData`),
    /// хотя в типе разбираемого выражения этот тег указан с данными (`SomeTyping`).
    #[expect(dead_code, reason = "WIP")]
    UnexpectedNullaryVariantPattern,

    // Stage 1 extra 7:
    //
    /// В объявлении функции повторяется имя параметра.
    DuplicateFunctionParameter {
        function: Cow<'src, str>,
        param: Ident<'src>,
    },
    /// Повторяющееся связывание в `let`-выражении.
    DuplicateLetBinding(Ident<'src>),
    /// Повторяющийся параметр типа.
    DuplicateTypeParameter(Ident<'src>),

    // Stage 2:
    //
    /// В программе используются исключения, но не объявлен их тип.
    #[expect(clippy::empty_enum_variants_with_brackets, reason = "intentional")]
    ExceptionTypeNotDeclared {},
    /// Неоднозначный тип `throw`-выражения (`Throw`).
    #[expect(clippy::empty_enum_variants_with_brackets, reason = "intentional")]
    AmbiguousThrowType {},
    /// Неоднозначный тип адреса памяти (`ConstMemory`).
    AmbiguousReferenceType,
    /// Неоднозначный тип ошибки (`Panic`).
    AmbiguousPanicType,
    /// Попытка разыменовать (`Deref`) или присвоить значение (`Assign`)
    /// выражению не ссылочного типа.
    NotAReference {
        expr: &'src Expr<'src>,
        actual: Type<'src>,
    },
    /// Адрес памяти (`ConstMemory`) используется там,
    /// где ожидается тип, отличный от типа-ссылки (`TypeRef`).
    UnexpectedMemoryAddress {
        address: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Выражение создания ссылки (`Ref`, т.е. `new(<выражение>)`)
    /// используется там, где ожидается не ссылочный тип.
    UnexpectedReference {
        reference: &'src Expr<'src>,
        expected: Type<'src>,
    },
    /// Тип выражения не является подтипом ожидаемого.
    ///
    /// Эта ошибка должна возникать только если ни одна из более специфичных ошибок
    /// (перечисленных выше вместе с кодами Этапа 1) не возникла раньше.
    UnexpectedSubtype {
        expected: Type<'src>,
        actual: Type<'src>,
        expr: Option<&'src Expr<'src>>,
    },

    // Stage 3:
    //
    /// Во время унификации возникает ограничение, порождающее (запрещённый) бесконечный тип.
    #[expect(dead_code, reason = "WIP")]
    OccursCheckInfiniteType,
    /// При попытке применить (`TypeApplication`) универсальное выражение к типовому аргументу,
    /// выражение оказывается не универсальной функцией.
    ///
    /// Ошибка должна возникать до проверки аргумента.
    #[expect(dead_code, reason = "WIP")]
    NotAGenericFunction,
    /// Вызов универсальной функции (`TypeApplication`)
    /// происходит с некорректным количеством типов-аргументов.
    #[expect(dead_code, reason = "WIP")]
    IncorrectNumberOfTypeArguments,
    /// В типе содержится необъявленная типовая переменная (только для `#universal-types`)
    UndefinedTypeVariable(Ident<'src>),

    // Custom:
    //
    UnsupportedExtensions(BTreeSet<&'src Extension>),
    MissingExtensions {
        mandatory: Extensions,
        choosable: BTreeSet<Extensions>,
    },

    DuplicateExceptionType {
        old: Type<'src>,
        new: Type<'src>,
    },
}

impl core::error::Error for Error<'_> {}

impl fmt::Display for Error<'_> {
    #[expect(clippy::too_many_lines, reason = "big `match`")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ERROR_{}", <&'static str>::from(self))?;

        match self {
            Self::MissingMain {} => writeln!(f, "`main` is missing or is not a function"),

            Self::MissingExtensions {
                mandatory,
                choosable,
            } => {
                writeln!(f, "the following extensions are required but not enabled:")?;
                for ext in mandatory {
                    writeln!(f, "\t{ext}")?
                }
                for options in choosable {
                    write!(f, "\tat least one of: ")?;
                    let first = &mut true;
                    for option in options {
                        if !mem::replace(first, false) {
                            write!(f, ", ")?
                        }
                        write!(f, "{option}")?
                    }
                    writeln!(f)?
                }
                Ok(())
            }
            Self::UnsupportedExtensions(extensions) => {
                writeln!(f, "the following unsupported extensions are enabled:")?;
                for ext in extensions {
                    writeln!(f, "\t{ext}")?
                }
                Ok(())
            }

            &Self::IncorrectArityOfMain(arity) => {
                const EXPECTED: usize = 1;
                debug_assert_ne!(
                    arity, EXPECTED,
                    "Arity of {EXPECTED} is not incorrect for `main`"
                );
                writeln!(
                    f,
                    "`main` has {arity} arguments, but is expected to have {EXPECTED}"
                )
            }

            Self::DuplicateFunctionParameter { function, param } => writeln!(
                f,
                "function\n\t{function}\nis declared with duplicates of parameter\n\t{param}"
            ),

            Self::DuplicateFunctionDeclaration(name) => {
                writeln!(f, "function `{name}` is defined multiple times")
            }
            Self::UndefinedVariable(name) => writeln!(f, "undefined variable `{name}`"),

            Self::UnexpectedTypeForParameter {
                actual,
                expected,
                param,
            } => writeln!(
                f,
                "parameter `{param}` has type\n\t{actual}\nbut expected to have type\n\t{expected}"
            ),

            Self::UnexpectedTypeForExpression {
                actual,
                expected,
                expr,
            } => writeln!(
                f,
                "expression\n\t{expr}\nhas type\n\t{actual}\nbut expected to have type\n\t{expected}"
            ),
            Self::UnexpectedSubtype {
                expected,
                actual,
                expr: Some(expr),
            } => writeln!(
                f,
                "expression\n\t{expr}\nhas type\n\t{actual}\nwhich is not a subtype of the expected type\n\t{expected}"
            ),
            Self::UnexpectedSubtype {
                expected,
                actual,
                expr: None,
            } => writeln!(f, "\t{actual}\nis not a subtype of\n\t{expected}"),

            Self::UnexpectedNumberOfParametersInLambda {
                expected,
                actual,
                lambda,
            } => writeln!(
                f,
                "abstraction\n\t{lambda}\nhas {actual} arguments, but is expected to have {expected}"
            ),

            Self::DuplicateRecordFields { expr, field } => {
                let Expr::Record(fields) = expr else {
                    unreachable!("{expr} is not a record")
                };
                debug_assert!(
                    fields
                        .iter()
                        .filter(|Binding { name, value: _ }| name == field)
                        .count()
                        >= 2,
                    "not a duplicate"
                );
                writeln!(f, "field `{field}` is declared more than once in\n\t{expr}")
            }
            Self::DuplicateRecordTypeFields(name)
            | Self::DuplicateVariantTypeFields(name)
            | Self::DuplicateLetBinding(name)
            | Self::DuplicateTypeParameter(name)
            | Self::DuplicateRecordPatternFields(name) => {
                writeln!(f, "`{name}` is declared more than once")
            }

            Self::UndefinedTypeVariable(name) => writeln!(f, "`{name}` not found"),

            Self::UnexpectedLambda { lambda, expected } => writeln!(
                f,
                "expected an abstraction\n\t{lambda}\nto be\n\t{expected}"
            ),

            Self::NotAFunction { expr, actual } => {
                debug_assert!(!matches!(actual, Type::Fn { .. }), "So *is* a function?");
                writeln!(
                    f,
                    "expression\n\t{expr}\nhas type\n\t{actual}\nbut expected to be a function"
                )
            }
            Self::NotAList { expr, actual } => {
                debug_assert!(!matches!(actual, Type::List(_)), "So *is* a list?");
                writeln!(
                    f,
                    "expression\n\t{expr}\nhas type\n\t{actual}\nbut expected to be a list"
                )
            }
            Self::NotARecord { expr, actual } => {
                debug_assert!(!matches!(actual, Type::Record(_)), "So *is* a record?");
                writeln!(
                    f,
                    "expression\n\t{expr}\nhas type\n\t{actual}\nbut expected to be a record"
                )
            }
            Self::NotATuple { expr, actual } => {
                debug_assert!(!matches!(actual, Type::Tuple(_)), "So *is* a tuple?");
                writeln!(
                    f,
                    "expression\n\t{expr}\nhas type\n\t{actual}\nbut expected to be a tuple"
                )
            }
            Self::NotAReference { expr, actual } => {
                debug_assert!(!matches!(actual, Type::Ref(_)), "So *is* a tuple?");
                writeln!(
                    f,
                    "expression\n\t{expr}\nhas type\n\t{actual}\nbut expected to be a reference"
                )
            }

            Self::IncorrectNumberOfArguments {
                expected,
                actual,
                callee,
            } => {
                debug_assert_ne!(actual, expected, "not incorrect");
                writeln!(
                    f,
                    "function\n\t{callee}\nis called with {actual} argument(s), but has arity of {expected}"
                )
            }
            Self::UnexpectedTupleLength {
                tuple,
                actual,
                expected,
            } => {
                debug_assert_matches!(tuple, Expr::Tuple(_));
                debug_assert_ne!(actual, expected, "not unexpected");
                writeln!(
                    f,
                    "tuple\n\t{tuple}\nhas {actual} elements(s), but expected to have {expected}"
                )
            }

            Self::UnexpectedList { list, expected } => {
                debug_assert_matches!(list, Expr::Cons { .. } | Expr::List(_));
                debug_assert!(
                    !matches!(expected, Type::List { .. }),
                    "So it's an *expected* list?"
                );
                writeln!(
                    f,
                    "expression\n\t{list}\nis a list, but expected to be\n\t{expected}"
                )
            }
            Self::UnexpectedInjection {
                injection,
                expected,
            } => {
                debug_assert_matches!(injection, Expr::Inl(_) | Expr::Inr(_));
                debug_assert!(
                    !matches!(expected, Type::Sum { .. }),
                    "So it's an *expected* injection?"
                );
                writeln!(
                    f,
                    "expression\n\t{injection}\nis an injections, but expected to be\n\t{expected}"
                )
            }
            Self::UnexpectedTuple { tuple, expected } => {
                debug_assert_matches!(tuple, Expr::Tuple(_));
                debug_assert!(
                    !matches!(expected, Type::Tuple(_)),
                    "So it's an *expected* tuple?"
                );
                writeln!(
                    f,
                    "expression\n\t{tuple}\nis a tuple, but expected to be\n\t{expected}"
                )
            }

            Self::UnexpectedRecord { record, expected } => {
                debug_assert_matches!(record, Expr::Record(_));
                debug_assert!(
                    !matches!(expected, Type::Record(_)),
                    "So it's an *expected* record?"
                );
                writeln!(
                    f,
                    "expression\n\t{record}\nis a record, but expected to be\n\t{expected}"
                )
            }

            Self::UnexpectedVariant { variant, expected } => {
                debug_assert_matches!(variant, Expr::Variant { .. });
                debug_assert!(
                    !matches!(expected, Type::Variant(_)),
                    "So it's an *expected* variant?"
                );
                writeln!(
                    f,
                    "expression\n\t{variant}\nis a variant, but expected to be\n\t{expected}"
                )
            }

            Self::UnexpectedMemoryAddress { address, expected } => {
                debug_assert_matches!(address, Expr::Address { .. });
                debug_assert!(!matches!(expected, Type::Ref(_)), "not unexpected");
                writeln!(
                    f,
                    "expression\n\t{address}\nis a memory address, but expected to be\n\t{expected}"
                )
            }
            Self::UnexpectedReference {
                reference,
                expected,
            } => {
                debug_assert_matches!(reference, Expr::New { .. });
                debug_assert!(!matches!(expected, Type::Ref(_)), "not unexpected");
                writeln!(
                    f,
                    "expression\n\t{reference}\nis a reference, but expected to be\n\t{expected}"
                )
            }

            Self::AmbiguousSumType(expr) => {
                debug_assert_matches!(expr, Expr::Inl(_) | Expr::Inr(_));
                writeln!(f, "cannot infer type of an injection\n\t{expr}")
            }
            Self::AmbiguousVariantType(expr) => {
                debug_assert_matches!(expr, Expr::Variant { .. });
                writeln!(f, "cannot infer type of variant\n\t{expr}")
            }

            &Self::TupleIndexOutOfBounds {
                tuple,
                length,
                index,
            } => {
                if let Ok(length) = Integer::try_from(length) {
                    debug_assert!(index == 0 || index > length, "So not out of bounds?");
                }
                writeln!(
                    f,
                    "index {index} is out of bounds for a tuple\n\t{tuple}\nof length {length}"
                )
            }
            Self::UnexpectedFieldAccess {
                record,
                expected,
                actual,
            } => {
                writeln!(
                    f,
                    "there is no field `{expected}` in\n\t{record}\nwhich is a record with the following fields:"
                )?;
                for (name, ty) in actual.as_ref() {
                    writeln!(f, "\t{name} : {ty}")?
                }
                Ok(())
            }

            Self::MissingRecordFields {
                actual: RecordWithMissingFields::Literal(actual),
                expected,
                missing,
            } => {
                debug_assert_matches!(actual, Expr::Record(_));
                debug_assert_matches!(expected, Type::Record(_));

                // TODO: debug_assert that field is missing
                writeln!(
                    f,
                    "expected\n\t{actual}\nto be\n\t{expected}\nbut it is missing field `{missing}`"
                )
            }
            Self::MissingRecordFields {
                actual:
                    RecordWithMissingFields::Subtyping {
                        actual_expr,
                        actual_type,
                    },
                expected,
                missing,
            } => {
                debug_assert_matches!(actual_type, Type::Record(_));
                debug_assert_matches!(expected, Type::Record(_));
                // TODO: debug_assert that field is missing
                writeln!(
                    f,
                    "expected\n{}\t{actual_type}\nto be a subtype of\n\t{expected}\nbut it is missing field `{missing}`",
                    fmt::from_fn(|f| {
                        if let Some(actual_expr) = actual_expr {
                            writeln!(f, "\t{actual_expr}\nwhich is")?
                        }
                        Ok(())
                    })
                )
            }
            Self::UnexpectedRecordFields {
                actual,
                expected,
                unexpected,
            } => {
                debug_assert_matches!(actual, Expr::Record(_));
                debug_assert_matches!(expected, Type::Record(_));
                // TODO: debug_assert that field is unexpected
                writeln!(
                    f,
                    "expected\n\t{actual}\nto be\n\t{expected}\nbut it has an unexpected field `{unexpected}`"
                )
            }
            Self::UnexpectedVariantLabel { expected, actual } => {
                let Type::Variant(options) = expected else {
                    unreachable!("not a variant: {expected:?}")
                };
                debug_assert!(
                    !options.iter().any(|(name, _)| name == actual),
                    "not unexpected"
                );
                writeln!(f, "there is no label `{actual}` in variant\n\t{expected}")
            }

            Self::UnexpectedDataForNullaryLabel { label, unexpected } => writeln!(
                f,
                "no data expected for label `{label}`, but found\n\t{unexpected}"
            ),
            Self::MissingDataForLabel { label, expected } => {
                writeln!(f, "no data for label `{label}`, but expected\n\t{expected}")
            }

            Self::AmbiguousReferenceType {} => writeln!(f, "cannot infer reference type"),
            Self::AmbiguousThrowType {} => writeln!(f, "cannot infer throw type"),
            Self::AmbiguousPanicType {} => writeln!(f, "cannot infer panic type"),
            Self::AmbiguousList {} => writeln!(f, "cannot infer element type of an empty list"),

            Self::ExceptionTypeNotDeclared {} => Ok(()),
            Self::DuplicateExceptionType { old, new } => writeln!(
                f,
                "redefinition of the exception type in the same scope\nold:\n\t{old}\nnew:\n\t{new}"
            ),

            Self::IllegalEmptyMatching
            | Self::NonexhaustiveMatchPatterns
            | Self::UnexpectedPatternForType
            | Self::UnexpectedNonNullaryVariantPattern
            | Self::UnexpectedNullaryVariantPattern
            | Self::OccursCheckInfiniteType
            | Self::NotAGenericFunction
            | Self::IncorrectNumberOfTypeArguments => todo!("format Error::{self:?}"),
        }
    }
}

macro_rules! nz {
    ($l:literal) => {
        const { NonZero::new($l).unwrap() }
    };
}

impl Error<'_> {
    #[must_use]
    pub(crate) const fn code(&self) -> NonZero<u8> {
        match self {
            Error::MissingMain {} => nz!(1),
            Error::UndefinedVariable(_) => nz!(2),
            Error::UnexpectedTypeForExpression { .. } => nz!(3),
            Error::NotAFunction { .. } => nz!(4),
            Error::NotATuple { .. } => nz!(5),
            Error::NotARecord { .. } => nz!(6),
            Error::NotAList { .. } => nz!(7),
            Error::UnexpectedLambda { .. } => nz!(8),
            Error::UnexpectedTypeForParameter { .. } => nz!(9),
            Error::UnexpectedTuple { .. } => nz!(10),
            Error::UnexpectedRecord { .. } => nz!(11),
            Error::UnexpectedVariant { .. } => nz!(12),
            Error::UnexpectedList { .. } => nz!(13),
            Error::UnexpectedInjection { .. } => nz!(14),
            Error::MissingRecordFields { .. } => nz!(15),
            Error::UnexpectedRecordFields { .. } => nz!(16),
            Error::UnexpectedFieldAccess { .. } => nz!(17),
            Error::UnexpectedVariantLabel { .. } => nz!(18),
            Error::TupleIndexOutOfBounds { .. } => nz!(19),
            Error::UnexpectedTupleLength { .. } => nz!(20),
            Error::AmbiguousSumType(_) => nz!(21),
            Error::AmbiguousVariantType(_) => nz!(22),
            Error::AmbiguousList {} => nz!(23),
            Error::IllegalEmptyMatching => nz!(24),
            Error::NonexhaustiveMatchPatterns => nz!(25),
            Error::UnexpectedPatternForType => nz!(26),
            Error::DuplicateRecordFields { .. } => nz!(27),
            Error::DuplicateRecordTypeFields(_) => nz!(28),
            Error::DuplicateVariantTypeFields(_) => nz!(29),
            Error::DuplicateFunctionDeclaration(_) => nz!(30),

            Error::IncorrectArityOfMain(_)
            | Error::IncorrectNumberOfArguments { .. }
            | Error::UnexpectedNumberOfParametersInLambda { .. } => nz!(113),
            Error::DuplicateRecordPatternFields(_) => nz!(114),
            Error::UnexpectedDataForNullaryLabel { .. }
            | Error::MissingDataForLabel { .. }
            | Error::UnexpectedNonNullaryVariantPattern
            | Error::UnexpectedNullaryVariantPattern => nz!(115),
            Error::DuplicateFunctionParameter { .. }
            | Error::DuplicateLetBinding(_)
            | Error::DuplicateTypeParameter(_) => nz!(117),

            Error::ExceptionTypeNotDeclared {} => nz!(31),
            Error::AmbiguousThrowType {} => nz!(32),
            Error::AmbiguousReferenceType => nz!(33),
            Error::AmbiguousPanicType => nz!(34),
            Error::NotAReference { .. } => nz!(35),
            Error::UnexpectedMemoryAddress { .. } => nz!(36),
            Error::UnexpectedReference { .. } => nz!(37),
            Error::UnexpectedSubtype { .. } => nz!(38),

            Error::OccursCheckInfiniteType => nz!(39),
            Error::NotAGenericFunction => nz!(40),
            Error::IncorrectNumberOfTypeArguments => nz!(41),
            Error::UndefinedTypeVariable(_) => nz!(42),

            Error::DuplicateExceptionType { .. } => nz!(252),
            Error::UnsupportedExtensions(_) => nz!(253),
            Error::MissingExtensions { .. } => nz!(254),
        }
    }
}
