

use std::{borrow::Borrow, collections::{HashMap, hash_map::{Drain, Iter}}, hash::Hash};

pub struct SeaMap<K, V>(HashMap<K, V, fasthash::sea::Hash64>);

#[allow(dead_code)]
impl<K, V> SeaMap<K, V>
where
    K: Eq + Hash,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SeaMap(HashMap::with_hasher(fasthash::sea::Hash64))
    }

    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.0.contains_key(k)
    }

    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.get(k)
    }

    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.get_mut(k)
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.insert(k, v)
    }

    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.remove(k)
    }
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        self.0.drain()
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        self.0.iter()
    }
}