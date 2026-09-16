use std::{borrow::Cow, collections::btree_set::BTreeSet};

#[derive(Debug)]
#[expect(dead_code, reason = "WIP")]
pub(crate) struct Program<'src> {
    pub extensions: BTreeSet<&'src str>,
    pub decls: Vec<Decl<'src>>,
}

pub(crate) type Integer = u128;
pub(crate) type Ident<'src> = Cow<'src, str>;

#[derive(Debug)]
#[expect(dead_code, reason = "WIP")]
pub(crate) struct PatternBinding<'src> {
    pub lhs: Pattern<'src>,
    pub rhs: Expr<'src>,
}

#[derive(Debug)]
#[expect(dead_code, reason = "WIP")]
pub(crate) struct Binding<'src> {
    pub lhs: Ident<'src>,
    pub rhs: Expr<'src>,
}

#[derive(Debug)]
#[expect(dead_code, reason = "WIP")]
pub(crate) struct Function<'src> {
    pub annotations: Vec<Annotation>,
    pub name: Ident<'src>,
    pub generics: Vec<Ident<'src>>,
    pub params: Vec<(Ident<'src>, Type<'src>)>,
    pub ret_ty: Option<Type<'src>>,
    pub throws: Vec<Type<'src>>,
    pub decls: Vec<Decl<'src>>,
    pub ret: Expr<'src>,
}

#[derive(Debug)]
#[expect(dead_code, reason = "WIP")]
pub(crate) enum Decl<'src> {
    Function(
        // Boxed to reduce `Decl` size from 336 to 72 bytes
        Box<Function<'src>>,
    ),
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
#[expect(dead_code, reason = "WIP")]
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
#[expect(dead_code, reason = "WIP")]
pub(crate) enum Pattern<'src> {
    CastAs {
        pat: Box<Self>,
        ty: Type<'src>,
    },
    Asc {
        pat: Box<Self>,
        ty: Type<'src>,
    },
    Variant {
        label: Ident<'src>,
        pat: Option<Box<Self>>,
    },
    Inl(Box<Self>),
    Inr(Box<Self>),
    Tuple(Vec<Self>),
    Record(Vec<(Ident<'src>, Self)>),
    List(Vec<Self>),
    Cons(Box<[Self; 2]>),
    False,
    True,
    Unit,
    Int(Integer),
    Succ(Box<Self>),
    Var(Ident<'src>),
}

pub(crate) type Catch<'src> = (Pattern<'src>, Box<Expr<'src>>);

#[derive(Debug)]
#[expect(dead_code, reason = "WIP")]
pub(crate) enum Expr<'src> {
    Seq(Box<[Self; 2]>),
    Let {
        rec: bool,
        bindings: Vec<PatternBinding<'src>>,
        body: Box<Self>,
    },
    Assign {
        lhs: Box<Self>,
        rhs: Box<Self>,
    },
    If {
        cond: Box<Self>,
        on_true: Box<Self>,
        on_false: Box<Self>,
    },
    Var(Ident<'src>),
    Generic {
        generics: Vec<Ident<'src>>,
        expr: Box<Self>,
    },
    BinOp {
        lhs: Box<Self>,
        op: BinOp,
        rhs: Box<Self>,
    },
    Ascription {
        expr: Box<Self>,
        ty: Type<'src>,
    },
    Cast {
        expr: Box<Self>,
        ty: Type<'src>,
    },

    Fn {
        params: Vec<(Ident<'src>, Type<'src>)>,
        ret: Box<Self>,
    },
    Variant {
        lhs: Ident<'src>,
        rhs: Option<Box<Self>>,
    },
    Match {
        switch: Box<Self>,
        cases: Vec<(Pattern<'src>, Self)>,
    },
    List(Vec<Self>),
    New(Box<Self>),
    Deref(Box<Self>),

    Application {
        lhs: Box<Self>,
        args: Vec<Self>,
    },
    TypeApplication {
        lhs: Box<Self>,
        types: Vec<Type<'src>>,
    },

    DotRecord {
        lhs: Box<Self>,
        field: Ident<'src>,
    },
    DotTuple {
        lhs: Box<Self>,
        field: Integer,
    },
    Tuple(Vec<Self>),
    Cons(Box<[Self; 2]>),
    Record(Vec<Binding<'src>>),
    ListHead(Box<Self>),
    ListIsEmpty(Box<Self>),
    ListTail(Box<Self>),

    Panic,
    Throw(Box<Self>),

    TryCatch {
        r#try: Box<Self>,
        catch: Catch<'src>,
    },

    TryWith {
        r#try: Box<Self>,
        with: Box<Self>,
    },
    TryCastAs {
        r#try: Box<Self>,
        r#as: Type<'src>,
        catch: Catch<'src>,
        with: Box<Self>,
    },
    Inl(Box<Self>),
    Inr(Box<Self>),
    Succ(Box<Self>),

    Not(Box<Self>),
    NatPred(Box<Self>),
    NatIsZero(Box<Self>),
    Fix(Box<Self>),
    NatRec(Box<[Self; 3]>),

    Fold {
        ty: Type<'src>,
        expr: Box<Self>,
    },
    Unfold {
        ty: Type<'src>,
        expr: Box<Self>,
    },

    True,
    False,
    Unit,
    Integer(Integer),
    Address(&'src str),
}

#[derive(Debug)]
pub(crate) enum BinOp {
    /// `<`
    LT,
    /// `<=`
    LE,
    /// `>`
    GT,
    /// `>=`
    GE,
    /// `==`
    EQ,
    /// `!=`
    NE,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `or`
    Or,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `and`
    And,
}
