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
//! [`with_slice`] does the same for indices: [`JSlice::check`] checks bounds
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
///
/// Sealed: a [`Key`] is only as trustworthy as the lookups that produced it,
/// so custom map types (whose `lookup`, `entries` and `size` could disagree)
/// are not accepted.
pub trait Map: crate::__private::SealedMap {
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

    impl<K, V> crate::__private::SealedMap for BTreeMap<K, V> {}
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

    impl<K, V, S> crate::__private::SealedMap for HashMap<K, V, S> {}
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

/// A borrowed map with a compile-time name `'ph`. A cheap `Copy` handle.
pub struct JMap<'ph, 'm, M> {
    map: &'m M,
    brand: Brand<'ph>,
}

impl<M> Clone for JMap<'_, '_, M> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M> Copy for JMap<'_, '_, M> {}

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

// Keys compare by the key they hold. Only keys of the same map (same 'ph)
// can be compared, so these are comparisons within one map.
impl<'ph, K: PartialEq, V> PartialEq for Key<'ph, '_, K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl<K: Eq, V> Eq for Key<'_, '_, K, V> {}
impl<'ph, K: PartialOrd, V> PartialOrd for Key<'ph, '_, K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.key.partial_cmp(other.key)
    }
}
impl<K: Ord, V> Ord for Key<'_, '_, K, V> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(other.key)
    }
}
impl<K: core::hash::Hash, V> core::hash::Hash for Key<'_, '_, K, V> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state)
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

impl<T> Clone for JSlice<'_, '_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for JSlice<'_, '_, T> {}

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

/// Iterator over every [`Index`] of a [`JSlice`], in order. It does not
/// borrow the slice, so it can be held while indexing.
#[derive(Clone, Debug)]
pub struct Indices<'ph> {
    range: core::ops::Range<usize>,
    brand: Brand<'ph>,
}

impl<'ph> Iterator for Indices<'ph> {
    type Item = Index<'ph>;

    #[inline]
    fn next(&mut self) -> Option<Index<'ph>> {
        self.range.next().map(mint)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl DoubleEndedIterator for Indices<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range.next_back().map(mint)
    }
}

impl ExactSizeIterator for Indices<'_> {}
impl core::iter::FusedIterator for Indices<'_> {}

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
    pub fn check(&self, i: usize) -> Option<Index<'ph>> {
        (i < self.slice.len()).then(|| mint(i))
    }

    /// The first index, if the slice is not empty.
    pub fn first(&self) -> Option<Index<'ph>> {
        self.check(0)
    }

    /// The last index, if the slice is not empty.
    pub fn last(&self) -> Option<Index<'ph>> {
        self.slice.len().checked_sub(1).map(mint)
    }

    /// The next index after `i`, if still in bounds.
    pub fn next(&self, i: Index<'ph>) -> Option<Index<'ph>> {
        self.check(i.index + 1)
    }

    /// The previous index before `i`, if any.
    pub fn prev(&self, i: Index<'ph>) -> Option<Index<'ph>> {
        i.index.checked_sub(1).map(mint)
    }

    /// Every index of the slice, in order.
    pub fn indices(&self) -> Indices<'ph> {
        Indices {
            range: 0..self.slice.len(),
            brand: PhantomData,
        }
    }

    /// Every element with its checked index, like `iter().enumerate()`.
    pub fn enumerate(
        &self,
    ) -> impl DoubleEndedIterator<Item = (Index<'ph>, &'s T)> + ExactSizeIterator + use<'ph, 's, T>
    {
        let slice = self.slice;
        self.indices().map(move |i| (i, &slice[i.index]))
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

/// Owned data aligned with a [`JSlice`]: one element per index, under the
/// same name `'ph`, so any [`Index<'ph>`] indexes it infallibly too.
///
/// Built only by [`JSlice::map`] and [`JSlice::try_map`], which is what
/// guarantees the lengths match. Use it for adjacency lists, per-element
/// results, or anything you would otherwise keep in a parallel `Vec`
/// indexed by raw `usize`.
///
/// ```
/// let words = ["gdp", "ghosts", "rust"];
/// gdp::justified::with_slice(&words, |w| {
///     let lens = w.map(|_, s| s.len());
///     let longest = w.indices().max_by_key(|&i| lens[i]).unwrap();
///     assert_eq!((w[longest], lens[longest]), ("ghosts", 6));
/// });
/// ```
#[cfg(feature = "alloc")]
pub struct Aligned<'ph, U> {
    items: alloc::vec::Vec<U>,
    brand: Brand<'ph>,
}

#[cfg(feature = "alloc")]
impl<'ph, 's, T> JSlice<'ph, 's, T> {
    /// Builds data aligned with this slice, one element per index.
    pub fn map<U>(&self, mut f: impl FnMut(Index<'ph>, &'s T) -> U) -> Aligned<'ph, U> {
        Aligned {
            items: self.indices().map(|i| f(i, self.get(i))).collect(),
            brand: PhantomData,
        }
    }

    /// Fallible [`JSlice::map`]: the place to do validation once, up front.
    pub fn try_map<U, E>(
        &self,
        mut f: impl FnMut(Index<'ph>, &'s T) -> Result<U, E>,
    ) -> Result<Aligned<'ph, U>, E> {
        Ok(Aligned {
            items: self
                .indices()
                .map(|i| f(i, self.get(i)))
                .collect::<Result<_, E>>()?,
            brand: PhantomData,
        })
    }
}

#[cfg(feature = "alloc")]
impl<'ph, U> Aligned<'ph, U> {
    /// The element at a checked index. Infallible.
    #[inline(always)]
    pub fn get(&self, i: Index<'ph>) -> &U {
        // In bounds: built with exactly one element per index of the slice
        // named 'ph, and never resized.
        &self.items[i.index]
    }

    /// Mutable access at a checked index. Infallible.
    #[inline(always)]
    pub fn get_mut(&mut self, i: Index<'ph>) -> &mut U {
        &mut self.items[i.index]
    }

    /// Number of elements (the length of the slice it is aligned with).
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether it is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterates over the elements in index order.
    pub fn iter(&self) -> core::slice::Iter<'_, U> {
        self.items.iter()
    }

    /// Iterates mutably over the elements in index order.
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, U> {
        self.items.iter_mut()
    }

    /// Builds new data aligned with the same slice.
    pub fn map<W>(&self, mut f: impl FnMut(Index<'ph>, &U) -> W) -> Aligned<'ph, W> {
        Aligned {
            items: self
                .items
                .iter()
                .enumerate()
                .map(|(i, u)| f(mint(i), u))
                .collect(),
            brand: PhantomData,
        }
    }

    /// Unwraps the data, dropping its alignment evidence.
    pub fn into_inner(self) -> alloc::vec::Vec<U> {
        self.items
    }
}

#[cfg(feature = "alloc")]
impl<'ph, U> ops::Index<Index<'ph>> for Aligned<'ph, U> {
    type Output = U;

    #[inline(always)]
    fn index(&self, i: Index<'ph>) -> &U {
        self.get(i)
    }
}

#[cfg(feature = "alloc")]
impl<'ph, U> ops::IndexMut<Index<'ph>> for Aligned<'ph, U> {
    #[inline(always)]
    fn index_mut(&mut self, i: Index<'ph>) -> &mut U {
        self.get_mut(i)
    }
}

#[cfg(feature = "alloc")]
impl<U: Clone> Clone for Aligned<'_, U> {
    fn clone(&self) -> Self {
        Aligned {
            items: self.items.clone(),
            brand: PhantomData,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, U> IntoIterator for &'a Aligned<'_, U> {
    type Item = &'a U;
    type IntoIter = core::slice::Iter<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[cfg(feature = "alloc")]
impl<'a, U> IntoIterator for &'a mut Aligned<'_, U> {
    type Item = &'a mut U;
    type IntoIter = core::slice::IterMut<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

#[cfg(feature = "alloc")]
impl<U> IntoIterator for Aligned<'_, U> {
    type Item = U;
    type IntoIter = alloc::vec::IntoIter<U>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(feature = "alloc")]
impl<U: fmt::Debug> fmt::Debug for Aligned<'_, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Aligned").field(&self.items).finish()
    }
}
