use crate::ast::{Annotation, Expr, Function, Type};

#[track_caller]
pub(crate) fn mu(ty: &Type<'_>) -> ! {
    debug_assert!(
        matches!(ty, Type::Rec { .. }),
        "This is not a recursive type"
    );
    unimplemented!("rec types are not supported")
}

#[track_caller]
pub(crate) fn fold_unfold(expr: &Expr<'_>) -> ! {
    debug_assert!(
        matches!(expr, Expr::Fold { .. } | Expr::Unfold { .. }),
        "This is not fold/unfold"
    );
    unimplemented!("fold/unfold are not supported")
}

#[track_caller]
pub(crate) fn annotations(_: &Annotation) -> ! {
    unimplemented!("functions with annotations are not supported")
}

#[track_caller]
pub(crate) fn throws(function: &Function<'_>) -> ! {
    debug_assert_ne!(function.throws, [], "This function does not throw");
    unimplemented!("functions that throw are not supported")
}

#[track_caller]
pub(crate) fn no_ret_ty(function: &Function<'_>) -> ! {
    debug_assert_eq!(
        function.ret_ty, None,
        "This function does have a return type"
    );
    unimplemented!("functions without an explicit return type are not supported")
}
