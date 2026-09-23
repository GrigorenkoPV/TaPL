use std::collections::btree_map::{BTreeMap, Entry};

#[derive(Debug)]
pub(crate) struct Namespace<'a, K: Ord, V> {
    parent: Option<&'a Self>,
    local: BTreeMap<K, V>,
}

impl<K: Ord, V> Default for Namespace<'_, K, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<K: Ord, V> Namespace<'_, K, V> {
    #[must_use]
    pub(crate) const fn empty() -> Self {
        Self {
            parent: None,
            local: BTreeMap::new(),
        }
    }

    #[must_use]
    #[expect(clippy::elidable_lifetime_names, reason = "Makes it clearer")]
    pub(crate) fn new_scope<'this>(&'this self) -> Namespace<'this, K, V> {
        Namespace {
            parent: if self.local.is_empty() {
                self.parent
            } else {
                Some(self)
            },
            local: BTreeMap::new(),
        }
    }

    /// [`BTreeMap::try_insert`] would be cool here
    pub(crate) fn add_local(&mut self, key: K, value: V) -> Result<(), (&V, V)> {
        match self.local.entry(key) {
            Entry::Vacant(e) => {
                let _: &mut V = e.insert(value);
                Ok(())
            }
            Entry::Occupied(e) => Err((e.into_mut(), value)),
        }
    }

    pub(crate) fn lookup(&self, key: &K) -> Option<&V> {
        self.local
            .get(key)
            .or_else(|| self.parent.and_then(|parent| parent.lookup(key)))
    }
}
