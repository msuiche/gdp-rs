//! Justified containers: map lookups that cannot miss.
//!
//! A port of Matt Noonan's `justified-containers`, the library that preceded
//! Ghosts of Departed Proofs. [`with_map`] names a borrowed map; a successful
//! [`JMap::member`] check returns a [`Key`] carrying evidence that the key is
//! present, and [`JMap::get`] on that key returns `&V`, not `Option<&V>`.
//!
//! ```
//! use std::collections::BTreeMap;
//! use gdp::justified::with_map;
//!
//! let ports = BTreeMap::from([("http", 80), ("https", 443)]);
//! let total = with_map(&ports, |ports| {
//!     let https = ports.member(&"https")?;   // the only fallible step
//!     Some(ports.get(https) + ports.get(https))
//! });
//! assert_eq!(total, Some(886));
//! ```
//!
//! Rust adds something Haskell needed persistence for: the map is *borrowed*
//! for the whole scope, so the borrow checker forbids removing a key while
//! evidence about it is alive. Evidence cannot go stale.
//!
//! A key from one map is rejected by another, even if the maps are equal:
//!
//! ```compile_fail
//! use std::collections::BTreeMap;
//! let a = BTreeMap::from([(1, "one")]);
//! let b = BTreeMap::from([(1, "uno")]);
//! gdp::justified::with_map(&a, |a| {
//!     gdp::justified::with_map(&b, |b| {
//!         let k = a.member(&1).unwrap();
//!         b.get(k); // error: `k` is a key of `a`
//!     })
//! });
//! ```

use alloc::collections::BTreeMap;
use core::borrow::Borrow;
use core::fmt;
use core::marker::PhantomData;

use crate::Brand;

/// A map that [`with_map`] can name. Implemented for `BTreeMap` and, with
/// the `std` feature, `HashMap`.
pub trait Lookup {
    /// Key type.
    type Key;
    /// Value type.
    type Value;

    /// Returns the stored key and value equal to `key`, if present.
    fn lookup<Q>(&self, key: &Q) -> Option<(&Self::Key, &Self::Value)>
    where
        Self::Key: Borrow<Q>,
        Q: Ord + core::hash::Hash + Eq + ?Sized;

    /// Looks up one of the map's own keys.
    fn lookup_own(&self, key: &Self::Key) -> Option<&Self::Value>;

    /// Iterates over all entries.
    fn entries(&self) -> impl Iterator<Item = (&Self::Key, &Self::Value)>;

    /// Number of entries.
    fn size(&self) -> usize;
}

impl<K: Ord, V> Lookup for BTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn lookup<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord + core::hash::Hash + Eq + ?Sized,
    {
        self.get_key_value(key)
    }

    fn lookup_own(&self, key: &K) -> Option<&V> {
        self.get(key)
    }

    fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.iter()
    }

    fn size(&self) -> usize {
        self.len()
    }
}

#[cfg(feature = "std")]
impl<K: core::hash::Hash + Eq, V, S: core::hash::BuildHasher> Lookup
    for std::collections::HashMap<K, V, S>
{
    type Key = K;
    type Value = V;

    fn lookup<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord + core::hash::Hash + Eq + ?Sized,
    {
        self.get_key_value(key)
    }

    fn lookup_own(&self, key: &K) -> Option<&V> {
        self.get(key)
    }

    fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.iter()
    }

    fn size(&self) -> usize {
        self.len()
    }
}

/// A borrowed map with a compile-time name `'ph`.
pub struct JMap<'ph, 'm, M> {
    map: &'m M,
    brand: Brand<'ph>,
}

/// A key with evidence that it is present in the map named `'ph`.
///
/// It holds a reference to the map's own copy of the key, so it is `Copy`
/// regardless of `K`.
pub struct Key<'ph, 'm, K> {
    key: &'m K,
    brand: Brand<'ph>,
}

impl<K> Clone for Key<'_, '_, K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Copy for Key<'_, '_, K> {}

impl<'m, K> Key<'_, 'm, K> {
    /// The key itself.
    pub fn key(&self) -> &'m K {
        self.key
    }
}

impl<K: fmt::Debug> fmt::Debug for Key<'_, '_, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Key").field(self.key).finish()
    }
}

/// Names `map` for the duration of `k`.
pub fn with_map<'m, M: Lookup, R>(map: &'m M, k: impl for<'ph> FnOnce(JMap<'ph, 'm, M>) -> R) -> R {
    k(JMap {
        map,
        brand: PhantomData,
    })
}

impl<'ph, 'm, M: Lookup> JMap<'ph, 'm, M> {
    /// Checks whether `key` is present. This is the one fallible step.
    pub fn member<Q>(&self, key: &Q) -> Option<Key<'ph, 'm, M::Key>>
    where
        M::Key: Borrow<Q>,
        Q: Ord + core::hash::Hash + Eq + ?Sized,
    {
        self.map.lookup(key).map(|(key, _)| Key {
            key,
            brand: PhantomData,
        })
    }

    /// Looks up a key known to be present. Infallible for honest keys.
    ///
    /// # Panics
    ///
    /// If the key's `Ord` or `Hash` changed through interior mutability since
    /// it was checked (e.g. `Cell` keys), which `std` documents as a logic
    /// error. It never returns the value of a different key.
    pub fn get(&self, key: Key<'ph, 'm, M::Key>) -> &'m M::Value {
        match self.map.lookup_own(key.key) {
            Some(value) => value,
            // The map is borrowed immutably for 'm and `key` was found in
            // this very map, so this is unreachable short of a key or hasher
            // whose `Ord` / `Hash` changed through interior mutability, or a
            // `Borrow` / `Ord` / `Hash` impl that is not a function of the key.
            None => unreachable!("justified key missing from its own map"),
        }
    }

    /// Every key of the map, each with evidence of membership.
    pub fn keys(&self) -> impl Iterator<Item = Key<'ph, 'm, M::Key>> + use<'ph, 'm, M> {
        self.map.entries().map(|(key, _)| Key {
            key,
            brand: PhantomData,
        })
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.map.size()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.size() == 0
    }

    /// The underlying map.
    pub fn inner(&self) -> &'m M {
        self.map
    }
}
