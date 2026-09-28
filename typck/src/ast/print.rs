use core::{
    fmt::{self, Display, Formatter, Result},
    mem,
};

use crate::{
    ast::{Binding, Expr, Type},
    extensions::unsupported,
};

fn paren_alt(f: &mut Formatter<'_>, cb: impl FnOnce(&mut Formatter<'_>) -> Result) -> Result {
    if f.alternate() {
        f.write_str("(")?
    }
    cb(f)?;
    if f.alternate() {
        f.write_str(")")?
    }
    Ok(())
}

fn comma_sep_with<T>(
    things: &[T],
    print: impl Fn(&mut Formatter<'_>, &T) -> Result,
) -> impl Display {
    fmt::from_fn(move |f| {
        let first = &mut true;
        for thing in things {
            if !mem::take(first) {
                f.write_str(", ")?
            }
            print(f, thing)?
        }
        Ok(())
    })
}
fn comma_sep(things: &[impl Display]) -> impl Display {
    comma_sep_with(things, |f, thing| write!(f, "{thing}"))
}

impl Display for Type<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // FIXME: less parens
        match self {
            Self::Bool => f.write_str("Bool"),
            Self::Nat => f.write_str("Nat"),
            Self::Unit => f.write_str("Unit"),
            Self::Top => f.write_str("Top"),
            Self::Bot => f.write_str("Bot"),
            Self::Auto => f.write_str("auto"),
            Self::Var(name) => f.write_str(name),
            Self::List(inner) => write!(f, "[{inner}]"),
            Self::Ref(inner) => write!(f, "&{inner:#}"),
            Self::Fn { args, ret } => {
                paren_alt(f, |f| write!(f, "fn({}) -> {ret:#}", comma_sep(args)))
            }
            Self::Tuple(elements) => write!(f, "{{{}}}", comma_sep(elements)),
            Self::Record(fields) => write!(
                f,
                "{{{}}}",
                comma_sep_with(fields, |f, (name, ty)| write!(f, "{name} : {ty}"))
            ),

            Self::Sum { left, right } => paren_alt(f, |f| write!(f, "{left:#} + {right:#}")),

            Self::Variant(options) => write!(
                f,
                "<|{}|>",
                comma_sep_with(options, |f, (name, data)| {
                    write!(f, "{name}")?;
                    if let Some(data) = data {
                        write!(f, ": {data}")?
                    }
                    Ok(())
                })
            ),

            Self::Forall { .. } => todo!("Display {self:?}"),

            Self::Rec { .. } => unsupported::mu(self),
        }
    }
}

impl Display for Binding<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let Self { name, value } = self;
        write!(f, "{name} = {value}")
    }
}

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Panic => f.write_str("panic!"),
            Self::True => f.write_str("true"),
            Self::False => f.write_str("false"),
            Self::Unit => f.write_str("unit"),
            Self::Var(name) => f.write_str(name),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Address(addr) => f.write_str(addr),

            Self::Abstraction { params, ret } => paren_alt(f, |f| {
                write!(
                    f,
                    "fn({}) {{ return {ret} }}",
                    comma_sep_with(params, |f, (name, ty)| write!(f, "{name}: {ty}"))
                )
            }),

            Self::If {
                cond,
                on_true,
                on_false,
            } => paren_alt(f, |f| {
                write!(f, "if {cond:#} then {on_true:#} else {on_false:#}")
            }),

            Self::Succ(i) => write!(f, "succ({i})"),
            Self::NatPred(expr) => write!(f, "Nat::pred({expr})"),
            Self::NatIsZero(expr) => write!(f, "Nat::iszero({expr})"),

            Self::Application { lhs, args } => write!(f, "{lhs:#}({})", comma_sep(args)),
            Self::Inl(expr) => write!(f, "inl({expr})"),
            Self::Inr(expr) => write!(f, "inr({expr})"),

            Self::Cons { head, tail } => write!(f, "cons({head}, {tail})"),
            Self::ListHead(list) => write!(f, "List::head({list})"),
            Self::ListTail(list) => write!(f, "List::tail({list})"),
            Self::ListIsEmpty(list) => write!(f, "List::isempty({list})"),
            Self::List(elements) => write!(f, "[{}]", comma_sep(elements)),
            Self::Tuple(elements) => write!(f, "{{{}}}", comma_sep(elements)),
            Self::Record(fields) => write!(f, "{{{}}}", comma_sep(fields)),

            Self::Variant { label, data } => {
                write!(f, "<| {label} ")?;
                if let Some(data) = data {
                    write!(f, "= {data} ")?
                }
                write!(f, "|>")
            }
            Self::New(expr) => write!(f, "new({expr})"),

            Self::Seq(_)
            | Self::Let { .. }
            | Self::Assign { .. }
            | Self::Generic { .. }
            | Self::BinOp { .. }
            | Self::Ascription { .. }
            | Self::Cast { .. }
            | Self::Match { .. }
            | Self::Deref(_)
            | Self::TypeApplication { .. }
            | Self::DotRecord { .. }
            | Self::DotTuple { .. }
            | Self::Throw(_)
            | Self::TryCatch { .. }
            | Self::TryWith { .. }
            | Self::TryCastAs { .. }
            | Self::Not(_)
            | Self::Fix(_)
            | Self::NatRec { .. } => todo!("Display {self:?}"),

            Self::Fold { .. } | Self::Unfold { .. } => unsupported::fold_unfold(self),
        }
    }
}
