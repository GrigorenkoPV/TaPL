use std::{borrow::Cow, collections::btree_set::BTreeSet};

#[derive(Debug)]
pub(crate) struct Program<'src> {
    pub extensions: BTreeSet<&'src str>,
}

pub(crate) type Ident<'src> = Cow<'src, str>;

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
