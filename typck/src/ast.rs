use std::collections::btree_set::BTreeSet;

#[derive(Debug)]
pub(crate) struct Program<'src> {
    pub extensions: BTreeSet<&'src str>,
}
