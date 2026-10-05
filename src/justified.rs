//! Justified containers: lookups that cannot miss.
//!
//! A port of Matt Noonan's `justified-containers`, the library that preceded
//! Ghosts of Departed Proofs.
//!
//! [`with_map`] names a borrowed map. A successful [`JMap::member`] check
//! returns a [`Key`] carrying evidence that the key is present, and
//! [`JMap::get`] on that key returns `&V`, not `Option<&V>`.
//!
//! ```
//! use std::collections::BTreeMap;
//! use gdp::justified::with_map;
//!
//! let ports = BTreeMap::from([("http", 80), ("https", 443)]);
//! let total = with_map(&ports, |ports| {
//!     let https = ports.member("https")?;   // the only fallible step
//!     Some(ports.get(https) + ports.get(https))
//! });
//! assert_eq!(total, Some(886));
//! ```
//!
//! [`with_slice`] does the same for indices: [`JSlice::index`] checks bounds
//! once, and indexing with the resulting [`Index`] cannot go out of bounds.
//!
//! Rust adds something Haskell needed persistence for: the container is
//! *borrowed* for the whole scope, so the borrow checker forbids inserting or
//! removing while evidence about it is alive.
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

use core::fmt;
use core::marker::PhantomData;
use core::ops;

use crate::Brand;

/// A map that [`with_map`] can name. Implemented for `BTreeMap` (feature
/// `alloc`) and `HashMap` (feature `std`).
pub trait Map {
    /// Key type.
    type Key;
    /// Value type.
    type Value;

    /// Iterates over all entries.
    fn entries(&self) -> impl Iterator<Item = (&Self::Key, &Self::Value)>;

    /// Number of entries.
    fn size(&self) -> usize;
}

/// Lookup by a borrowed form `Q` of the key, with the bounds each map
/// actually needs: `Ord` for `BTreeMap`, `Hash + Eq` for `HashMap`.
pub trait Lookup<Q: ?Sized>: Map {
    /// Returns the stored key and value equal to `key`, if present.
    fn lookup(&self, key: &Q) -> Option<(&Self::Key, &Self::Value)>;
}

#[cfg(feature = "alloc")]
mod btree {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::borrow::Borrow;

    impl<K, V> Map for BTreeMap<K, V> {
        type Key = K;
        type Value = V;

        fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
            self.iter()
        }

        fn size(&self) -> usize {
            self.len()
        }
    }

    impl<K: Ord + Borrow<Q>, V, Q: Ord + ?Sized> Lookup<Q> for BTreeMap<K, V> {
        fn lookup(&self, key: &Q) -> Option<(&K, &V)> {
            self.get_key_value(key)
        }
    }
}

#[cfg(feature = "std")]
mod hash {
    use super::*;
    use core::borrow::Borrow;
    use core::hash::{BuildHasher, Hash};
    use std::collections::HashMap;

    impl<K, V, S> Map for HashMap<K, V, S> {
        type Key = K;
        type Value = V;

        fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
            self.iter()
        }

        fn size(&self) -> usize {
            self.len()
        }
    }

    impl<K, V, S, Q> Lookup<Q> for HashMap<K, V, S>
    where
        K: Hash + Eq + Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        S: BuildHasher,
    {
        fn lookup(&self, key: &Q) -> Option<(&K, &V)> {
            self.get_key_value(key)
        }
    }
}

/// A borrowed map with a compile-time name `'ph`.
pub struct JMap<'ph, 'm, M> {
    map: &'m M,
    brand: Brand<'ph>,
}

/// A key with evidence that it is present in the map named `'ph`.
///
/// It holds references to the map's own entry, captured by the check, so
/// [`JMap::get`] is a field read: no second lookup, and no way to fail even
/// if the key type's `Ord` / `Hash` misbehaves later.
pub struct Key<'ph, 'm, K, V> {
    key: &'m K,
    value: &'m V,
    brand: Brand<'ph>,
}

impl<K, V> Clone for Key<'_, '_, K, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V> Copy for Key<'_, '_, K, V> {}

impl<'m, K, V> Key<'_, 'm, K, V> {
    /// The key itself, as stored in the map.
    pub fn key(&self) -> &'m K {
        self.key
    }
}

impl<K: fmt::Debug, V> fmt::Debug for Key<'_, '_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Key").field(self.key).finish()
    }
}

/// Names `map` for the duration of `k`.
pub fn with_map<'m, M: Map, R>(map: &'m M, k: impl for<'ph> FnOnce(JMap<'ph, 'm, M>) -> R) -> R {
    k(JMap {
        map,
        brand: PhantomData,
    })
}

impl<'ph, 'm, M: Map> JMap<'ph, 'm, M> {
    /// Checks whether `key` is present. This is the one fallible step.
    pub fn member<Q: ?Sized>(&self, key: &Q) -> Option<Key<'ph, 'm, M::Key, M::Value>>
    where
        M: Lookup<Q>,
    {
        self.map.lookup(key).map(|(key, value)| Key {
            key,
            value,
            brand: PhantomData,
        })
    }

    /// Looks up a key known to be present. Infallible.
    #[inline(always)]
    pub fn get(&self, key: Key<'ph, 'm, M::Key, M::Value>) -> &'m M::Value {
        key.value
    }

    /// Every entry of the map, each key with evidence of membership.
    pub fn keys(&self) -> impl Iterator<Item = Key<'ph, 'm, M::Key, M::Value>> + use<'ph, 'm, M> {
        self.map.entries().map(|(key, value)| Key {
            key,
            value,
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

impl<M: fmt::Debug> fmt::Debug for JMap<'_, '_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JMap").field(self.map).finish()
    }
}

/// A borrowed slice with a compile-time name `'ph`.
///
/// ```
/// let scores = [90, 72, 85];
/// let best = gdp::justified::with_slice(&scores, |s| {
///     let best = s.indices().max_by_key(|&i| s[i])?;
///     Some((best.get(), s[best]))
/// });
/// assert_eq!(best, Some((0, 90)));
/// ```
pub struct JSlice<'ph, 's, T> {
    slice: &'s [T],
    brand: Brand<'ph>,
}

/// An index with evidence that it is in bounds for the slice named `'ph`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index<'ph> {
    index: usize,
    brand: Brand<'ph>,
}

impl Index<'_> {
    /// The raw index.
    #[inline(always)]
    pub fn get(self) -> usize {
        self.index
    }
}

impl fmt::Debug for Index<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Index").field(&self.index).finish()
    }
}

/// Iterator over every [`Index`] of a [`JSlice`]. It does not borrow the
/// slice, so it can be held while indexing.
pub type Indices<'ph> = core::iter::Map<core::ops::Range<usize>, fn(usize) -> Index<'ph>>;

/// Names `slice` for the duration of `k`.
pub fn with_slice<'s, T, R>(slice: &'s [T], k: impl for<'ph> FnOnce(JSlice<'ph, 's, T>) -> R) -> R {
    k(JSlice {
        slice,
        brand: PhantomData,
    })
}

#[inline(always)]
fn mint<'ph>(index: usize) -> Index<'ph> {
    Index {
        index,
        brand: PhantomData,
    }
}

impl<'ph, 's, T> JSlice<'ph, 's, T> {
    /// Checks that `i` is in bounds. This is the one fallible step.
    #[inline]
    pub fn index(&self, i: usize) -> Option<Index<'ph>> {
        (i < self.slice.len()).then(|| mint(i))
    }

    /// The first index, if the slice is not empty.
    pub fn first(&self) -> Option<Index<'ph>> {
        self.index(0)
    }

    /// The last index, if the slice is not empty.
    pub fn last(&self) -> Option<Index<'ph>> {
        self.slice.len().checked_sub(1).map(mint)
    }

    /// The next index after `i`, if still in bounds.
    pub fn next(&self, i: Index<'ph>) -> Option<Index<'ph>> {
        self.index(i.index + 1)
    }

    /// The previous index before `i`, if any.
    pub fn prev(&self, i: Index<'ph>) -> Option<Index<'ph>> {
        i.index.checked_sub(1).map(mint)
    }

    /// Every index of the slice, in order.
    pub fn indices(&self) -> Indices<'ph> {
        (0..self.slice.len()).map(mint)
    }

    /// The element at a checked index. Infallible.
    #[inline(always)]
    pub fn get(&self, i: Index<'ph>) -> &'s T {
        // In bounds: `i` was checked against this slice, whose length cannot
        // change while it is borrowed.
        &self.slice[i.index]
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.slice.len()
    }

    /// Whether the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    /// The underlying slice.
    pub fn inner(&self) -> &'s [T] {
        self.slice
    }
}

impl<'ph, T> ops::Index<Index<'ph>> for JSlice<'ph, '_, T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, i: Index<'ph>) -> &T {
        self.get(i)
    }
}

impl<T: fmt::Debug> fmt::Debug for JSlice<'_, '_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JSlice").field(&self.slice).finish()
    }
}
