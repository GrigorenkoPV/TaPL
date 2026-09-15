use std::{borrow::Cow, collections::btree_set::BTreeSet};

#[derive(Debug)]
pub(crate) struct Program<'src> {
    pub extensions: BTreeSet<&'src str>,
    pub decls: Vec<Decl<'src>>,
}

pub(crate) type Ident<'src> = Cow<'src, str>;

#[derive(Debug)]
pub(crate) enum Decl<'src> {
    Function {
        annotations: Vec<Annotation>,
        name: Ident<'src>,
        generics: Vec<Ident<'src>>,
        params: Vec<(Ident<'src>, Type<'src>)>,
        ret_ty: Option<Type<'src>>,
        throws: Vec<Type<'src>>,
        decls: Vec<Self>,
        ret: Expr<'src>,
    },
    TypeAlias {
        name: Ident<'src>,
        ty: Type<'src>,
    },
    ExceptionType(Type<'src>),
    ExceptionVariant {
        name: Ident<'src>,
        ty: Type<'src>,
    },
}

#[derive(Debug)]
pub(crate) enum Annotation {
    Inline,
}

#[derive(Debug)]
pub(crate) enum Type<'src> {
    Bool,
    Nat,
    Ref(Box<Self>),
    Sum {
        left: Box<Self>,
        right: Box<Self>,
    },
    Fn {
        args: Vec<Self>,
        ret: Box<Self>,
    },
    Forall {
        binders: Vec<Ident<'src>>,
        ty: Box<Self>,
    },
    Rec {
        var: Ident<'src>,
        ty: Box<Self>,
    },
    Tuple(Vec<Self>),
    Record(Vec<(Ident<'src>, Self)>),
    Variant(Vec<(Ident<'src>, Option<Self>)>),
    List(Box<Self>),
    Unit,
    Top,
    Bot,
    Auto,
    Var(Ident<'src>),
}

#[derive(Debug)]
pub(crate) enum Expr<'src> {
    Var(Ident<'src>),
}
