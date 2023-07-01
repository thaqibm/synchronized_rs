use std::{borrow::Borrow, hash::Hash};


pub mod densemap;
pub trait SyncMap<K,V> {
    fn insert(&mut self, k: K, v: V) -> Option<V>;

    fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq;

    fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq;
}
