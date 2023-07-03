use core::hash::{BuildHasher, Hash, Hasher};
use std::{
    collections::hash_map::RandomState,
    marker::PhantomData,
    sync::{Mutex, RwLock},
};

use super::SyncMap;
use crate::util::{Counter, PadBytes};

enum EntryState<K, V> {
    Empty,
    Occupied { key: K, value: V },
    Tombstone,
}

struct LockDenseMap<K, V, S = RandomState> {
    _padding_begin: PadBytes<64>,
    num_threads: u32,
    data: Box<[RwLock<EntryState<K, V>>]>,
    old_data: Box<[RwLock<EntryState<K, V>>]>,
    approx_inserts: Box<Counter>,
    approx_deletes: Box<Counter>,
    _padding_end: PadBytes<64>,
    hasher: S,
}

unsafe impl<K, V, S> Send for LockDenseMap<K, V, S> {}
unsafe impl<K, V, S> Sync for LockDenseMap<K, V, S> {}

impl<K, V, S> LockDenseMap<K, V, S> {
    const PROBE_LIMIT: usize = 100;

    fn expand_needed(&self, probe_count: usize) -> bool {
        let approx = self.approx_inserts.get() > (self.data.len() / 3) as u64;
        approx
            || (probe_count > Self::PROBE_LIMIT
                && self.approx_inserts.get_accurate() > (self.data.len() / 3) as u64)
    }

    fn _insert(&self, key: K, value: V) -> Option<V> {
        todo!()
    }
}

impl<K, V> SyncMap<K, V> for LockDenseMap<K, V> {
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        todo!()
    }

    fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq,
    {
        todo!()
    }

    fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq,
    {
        todo!()
    }
}
