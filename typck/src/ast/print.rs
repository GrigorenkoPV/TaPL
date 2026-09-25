use core::{
    fmt::{Display, Formatter, Result},
    mem,
};

use crate::{
    ast::{Expr, Type},
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
            Self::Fn { args, ret } => paren_alt(f, |f| {
                f.write_str("fn(")?;
                let first = &mut true;
                for arg in args {
                    if !mem::take(first) {
                        f.write_str(", ")?
                    }
                    write!(f, "{arg}")?
                }

                write!(f, ") -> {ret:#}")
            }),
            Self::Sum { .. }
            | Self::Forall { .. }
            | Self::Rec { .. }
            | Self::Tuple(..)
            | Self::Record(..)
            | Self::Variant(..) => todo!("Display {self:?}"),
        }
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
                write!(f, "fn(")?;
                let first = &mut true;
                for (name, ty) in params {
                    if !mem::take(first) {
                        write!(f, ", ")?
                    }
                    write!(f, "{name}: {ty}")?
                }

                write!(f, ") {{ return {ret} }}")
            }),

            Self::If {
                cond,
                on_true,
                on_false,
            } => paren_alt(f, |f| {
                write!(f, "if {cond:#} then {on_true:#} else {on_false:#}")
            }),

            Self::Succ(i) => write!(f, "succ({i})"),

            Self::Seq(_)
            | Self::Let { .. }
            | Self::Assign { .. }
            | Self::Generic { .. }
            | Self::BinOp { .. }
            | Self::Ascription { .. }
            | Self::Cast { .. }
            | Self::Variant { .. }
            | Self::Match { .. }
            | Self::List(_)
            | Self::New(_)
            | Self::Deref(_)
            | Self::Application { .. }
            | Self::TypeApplication { .. }
            | Self::DotRecord { .. }
            | Self::DotTuple { .. }
            | Self::Tuple(_)
            | Self::Cons(_)
            | Self::Record(_)
            | Self::ListHead(_)
            | Self::ListIsEmpty(_)
            | Self::ListTail(_)
            | Self::Throw(_)
            | Self::TryCatch { .. }
            | Self::TryWith { .. }
            | Self::TryCastAs { .. }
            | Self::Inl(_)
            | Self::Inr(_)
            | Self::Not(_)
            | Self::NatPred(_)
            | Self::NatIsZero(_)
            | Self::Fix(_)
            | Self::NatRec { .. } => todo!("Display {self:?}"),

            Self::Fold { .. } | Self::Unfold { .. } => unsupported::fold_unfold(self),
        }
    }
}
