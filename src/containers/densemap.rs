use super::SyncMap;
use crate::util::{Counter, PadBytes, MAX_THREADS, PADDING_BYTES};
use std::borrow::Borrow;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::{collections::hash_map::RandomState, sync::RwLock};

const LENGTH_MULTIPLIER: usize = 4;
const MIN_SIZE: usize = 8;
const PROBE_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug)]
enum Bucket<K, V> {
    Empty,
    Occupied { key: K, value: V },
    Tombstone,
}

impl<K, V> Bucket<K, V> {
    pub fn is_empty(&self) -> bool {
        if let Bucket::Empty = self {
            true
        } else {
            false
        }
    }

    pub fn has_value(&self) -> bool {
        if let Bucket::Occupied { key: _, value: _ } = self {
            true
        } else {
            false
        }
    }

    pub fn is_tombstone(&self) -> bool {
        if let Bucket::Tombstone = self {
            true
        } else {
            false
        }
    }

    pub fn value(self) -> Option<V> {
        match self {
            Bucket::Occupied { key: _, value } => Some(value),
            _ => None,
        }
    }

    pub fn value_ref(&self) -> Result<&V, ()> {
        if let Bucket::Occupied { key: _, ref value } = *self {
            Ok(value)
        } else {
            Err(())
        }
    }

    pub fn key_eq(&self, k: &K) -> bool
    where
        K: PartialEq,
    {
        if let Bucket::Occupied { ref key, value: _ } = self {
            key == k
        } else {
            false
        }
    }
}

struct LockDenseTable<K, V, S = RandomState> {
    _padding_begin: PadBytes<64>,
    num_threads: u32,
    data: Box<[RwLock<Bucket<K, V>>]>,
    approx_inserts: Box<Counter>,
    approx_deletes: Box<Counter>,
    _padding_end: PadBytes<64>,
    hasher: S,
}

unsafe impl<K, V, S> Send for LockDenseTable<K, V, S> {}
unsafe impl<K, V, S> Sync for LockDenseTable<K, V, S> {}

impl<K, V> LockDenseTable<K, V> {
    pub fn new(size: usize) -> Self {
        let table = (0..size).map(|_| RwLock::new(Bucket::Empty)).collect();

        Self {
            _padding_begin: [0; PADDING_BYTES],
            num_threads: MAX_THREADS as u32,
            data: table,
            approx_inserts: Box::new(Counter::new(MAX_THREADS as u64)),
            approx_deletes: Box::new(Counter::new(MAX_THREADS as u64)),
            _padding_end: [0; PADDING_BYTES],
            hasher: RandomState::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self::new(min(MIN_SIZE, cap * LENGTH_MULTIPLIER))
    }
}

impl<K: PartialEq + Hash, V> LockDenseTable<K, V> {
    fn hash<T: ?Sized + Hash>(&self, key: &T) -> usize {
        let mut hasher = self.hasher.build_hasher();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn scan<F, Q: ?Sized>(&self, key: &Q, pred: F) -> RwLockReadGuard<Bucket<K, V>>
    where
        F: Fn(&Bucket<K, V>) -> bool,
        K: Borrow<Q>,
        Q: Hash,
    {
        let hash = self.hash(key);
        for i in 0..self.data.len() {
            let read = self.data[(hash + i) % self.data.len()].read().unwrap();
            if pred(&read) {
                return read;
            }
        }
        panic!("LockDenseMap scan failed");
    }

    fn scan_mut<F, Q: ?Sized>(&self, key: &Q, pred: F) -> RwLockWriteGuard<Bucket<K, V>>
    where
        F: Fn(&Bucket<K, V>) -> bool,
        K: Borrow<Q>,
        Q: Hash,
    {
        let hash = self.hash(key);
        for i in 0..self.data.len() {
            let write = self.data[(hash + i) % self.data.len()].write().unwrap();
            if pred(&write) {
                return write;
            }
        }
        panic!("LockDenseMap scan_mut failed");
    }

    fn scan_mut_no_lock<F>(&mut self, key: &K, pred: F) -> &mut Bucket<K, V>
    where
        F: Fn(&Bucket<K, V>) -> bool,
    {
        let hash = self.hash(key);
        let len = self.data.len();
        for i in 0..self.data.len() {
            let index = (hash + i) % len;
            let bucket = self.data[index].get_mut().unwrap();
            if pred(&bucket) {
                return self.data[index].get_mut().unwrap();
            }
        }
        panic!("`LockDenseMap` scan_mut_no_lock failed! No entry found.");
    }

    fn lookup_or_free(&self, key: &K) -> RwLockWriteGuard<Bucket<K, V>> {
        let hash = self.hash(key);

        let mut free = None;

        for i in 0..self.data.len() {
            let lock = self.data[(hash + i) % self.data.len()].write().unwrap();

            if lock.key_eq(key) {
                return lock;
            } else if lock.is_empty() {
                return free.unwrap_or(lock);
            } else if lock.is_tombstone() && free.is_none() {
                free = Some(lock)
            }
        }
        free.expect("No free buckets found")
    }

    fn lookup<Q: ?Sized>(&self, key: &Q) -> RwLockReadGuard<Bucket<K, V>>
    where
        K: Borrow<Q>,
        Q: PartialEq + Hash,
    {
        self.scan(key, |x| match *x {
            Bucket::Occupied {
                key: ref candidate_key,
                value: _,
            } if key.eq(candidate_key.borrow()) => true,
            Bucket::Empty => true,
            _ => false,
        })
    }

    fn lookup_mut<Q: ?Sized>(&self, key: &Q) -> RwLockWriteGuard<Bucket<K, V>>
    where
        K: Borrow<Q>,
        Q: PartialEq + Hash,
    {
        self.scan_mut(key, |x| match *x {
            Bucket::Occupied {
                key: ref candidate_key,
                value: _,
            } if key.eq(candidate_key.borrow()) => true,
            Bucket::Empty => true,
            _ => false,
        })
    }

    fn find_free(&self, key: &K) -> RwLockWriteGuard<Bucket<K, V>> {
        self.scan_mut(key, |x| x.is_empty())
    }

    fn find_free_no_lock(&mut self, key: &K) -> &mut Bucket<K, V> {
        self.scan_mut_no_lock(key, |x| x.is_empty())
    }
}

impl<K: Clone, V: Clone> Clone for LockDenseTable<K, V> {
    fn clone(&self) -> Self {
        let mut table = Self::new(self.data.len());
        table.data = self
            .data
            .iter()
            .map(|x| RwLock::new(x.read().unwrap().clone()))
            .collect();
        table.hasher = self.hasher.clone();
        table.num_threads = self.num_threads;
        table.approx_inserts = self.approx_inserts.clone();
        table.approx_deletes = self.approx_deletes.clone();

        table
    }
}

impl<K: Debug, V: Debug> Debug for LockDenseTable<K, V> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let mut map = f.debug_map();
        for i in self.data.iter() {
            let lock = i.read().unwrap();
            if let Bucket::Occupied { ref key, ref value } = *lock {
                map.entry(key, value);
            }
        }
        map.finish()
    }
}

pub struct IntoIter<K, V> {
    /// The inner table.
    items: Vec<RwLock<Bucket<K, V>>>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        // We own the table, and can thus do what we want with it. We'll simply pop from the
        // buckets until we find a bucket containing data.
        while let Some(bucket) = self.items.pop() {
            // We can bypass dem ebil locks.
            if let Bucket::Occupied { key, value } = bucket.into_inner().unwrap() {
                // The bucket contained data, so we'll return the pair.
                return Some((key, value));
            }
        }
        // We've exhausted all the buckets, and no more data could be found.
        None
    }
}

impl<K, V> IntoIterator for LockDenseTable<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter {
            items: self.data.into_vec(),
        }
    }
}
