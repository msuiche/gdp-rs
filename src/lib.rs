//! # gdp: Ghosts of Departed Proofs for Rust
//!
//! Turn "did we check that this user may touch this project?" into a compile
//! error instead of an incident.
//!
//! Three ideas, in order:
//!
//! 1. [`name`] gives a runtime value a compile-time-only *name*: a fresh,
//!    invariant lifetime `'id` that exists only inside a closure. Every call
//!    produces a different `'id`, and nothing branded with it can leave the
//!    closure.
//!
//! 2. [`proof!`] declares a proof type. Its constructor is private to the
//!    module that declares it, so that module (the *trusted module*) is the
//!    only place that can produce one. The compiler enforces this: there is no
//!    `as` cast, no lint, and no convention involved.
//!
//! 3. Sensitive functions take a proof about their exact named arguments.
//!    Calling them with no proof, a raw id, or a proof about a different value
//!    does not compile.
//!
//! Proofs are zero-sized and `Copy`. Names are a lifetime parameter. At
//! runtime there is nothing left but the values you named.
//!
//! ```
//! mod proofs {
//!     use gdp::Named;
//!     pub struct UserId(pub u64);
//!     pub struct ProjectId(pub u64);
//!
//!     gdp::proof! {
//!         /// The user is an Owner or Member of the project.
//!         pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId);
//!     }
//!
//!     pub fn user_is_project_admin<'u, 'p>(
//!         user: &Named<'u, UserId>,
//!         project: &Named<'p, ProjectId>,
//!     ) -> Option<UserIsProjectAdmin<'u, 'p>> {
//!         let is_admin = user.0 == 1 && project.0 == 42; // your real check
//!         is_admin.then(|| UserIsProjectAdmin::prove(user, project))
//!     }
//! }
//!
//! use gdp::Named;
//! use proofs::*;
//!
//! fn disable_password_protection<'u, 'p>(
//!     project: &Named<'p, ProjectId>,
//!     _proof: UserIsProjectAdmin<'u, 'p>,
//! ) -> u64 {
//!     project.0
//! }
//!
//! let disabled = gdp::name2(UserId(1), ProjectId(42), |user, project| {
//!     let admin = proofs::user_is_project_admin(&user, &project)?;
//!     Some(disable_password_protection(&project, admin))
//! });
//! assert_eq!(disabled, Some(42));
//! ```
//!
//! The same call with the arguments swapped, with a proof about another
//! project, or without the check, is rejected by the compiler:
//!
//! ```compile_fail
//! # mod proofs {
//! #     use gdp::Named;
//! #     pub struct UserId(pub u64);
//! #     pub struct ProjectId(pub u64);
//! #     gdp::proof! { pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId); }
//! #     pub fn user_is_project_admin<'u, 'p>(u: &Named<'u, UserId>, p: &Named<'p, ProjectId>)
//! #         -> Option<UserIsProjectAdmin<'u, 'p>> { Some(UserIsProjectAdmin::prove(u, p)) }
//! # }
//! # use gdp::Named;
//! # use proofs::*;
//! # fn disable_password_protection<'u, 'p>(_: &Named<'p, ProjectId>, _: UserIsProjectAdmin<'u, 'p>) {}
//! gdp::name3(UserId(1), ProjectId(42), ProjectId(7), |user, mine, theirs| {
//!     let admin = proofs::user_is_project_admin(&user, &mine).unwrap();
//!     disable_password_protection(&theirs, admin); // proof is about `mine`
//! });
//! ```
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod justified;

use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

/// The phantom carrier of a name.
///
/// `fn(&'id ()) -> &'id ()` is invariant in `'id`: a longer or shorter
/// lifetime cannot stand in for it, so two names never unify. Function
/// pointers are `Send + Sync + Copy`, so branding never costs you thread
/// safety or `Copy`.
#[doc(hidden)]
pub mod __private {
    /// Seals [`crate::Proof`]. Implemented by the `proof!` and `policy!`
    /// expansions. Not public API: implementing it by hand is forging.
    pub trait Sealed {}
}

#[doc(hidden)]
pub type Brand<'id> = PhantomData<fn(&'id ()) -> &'id ()>;

/// A value of type `T` tagged with the compile-time-only name `'id`.
///
/// The only way to obtain one is through [`name`] and its siblings. Read the
/// value through `Deref`, [`Named::value`], or [`Named::into_inner`].
///
/// `Named` deliberately has no `map`: a proof is about *this* value, and a
/// transformed value would need a proof of its own.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Named<'id, T> {
    value: T,
    brand: Brand<'id>,
}

impl<'id, T> Named<'id, T> {
    #[inline(always)]
    const fn new(value: T) -> Self {
        Named {
            value,
            brand: PhantomData,
        }
    }

    /// Borrows the underlying value.
    #[inline(always)]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Unwraps the underlying value, dropping its name.
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for Named<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> AsRef<T> for Named<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for Named<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Named").field(&self.value).finish()
    }
}

/// Gives `a` a fresh name for the duration of `k`.
///
/// `k` must work for *every* lifetime `'id`, so inside it `'id` is opaque and
/// unique. `R` cannot mention `'id`, which is what keeps names and proofs from
/// escaping:
///
/// ```compile_fail
/// let leaked = gdp::name(1u32, |n| n); // error: lifetime may not live long enough
/// ```
#[inline(always)]
pub fn name<A, R>(a: A, k: impl for<'a> FnOnce(Named<'a, A>) -> R) -> R {
    k(Named::new(a))
}

/// Gives two values two distinct fresh names for the duration of `k`.
#[inline(always)]
pub fn name2<A, B, R>(
    a: A,
    b: B,
    k: impl for<'a, 'b> FnOnce(Named<'a, A>, Named<'b, B>) -> R,
) -> R {
    k(Named::new(a), Named::new(b))
}

/// Gives three values three distinct fresh names for the duration of `k`.
#[inline(always)]
pub fn name3<A, B, C, R>(
    a: A,
    b: B,
    c: C,
    k: impl for<'a, 'b, 'c> FnOnce(Named<'a, A>, Named<'b, B>, Named<'c, C>) -> R,
) -> R {
    k(Named::new(a), Named::new(b), Named::new(c))
}

/// Async [`name`]: the body is an async closure, so it can `.await` checks
/// and sensitive calls while names stay scoped to it.
///
/// ```
/// # async fn demo() -> u32 {
/// gdp::name_async(7u32, async |n| *n + 1).await
/// # }
/// ```
pub async fn name_async<A, R>(a: A, k: impl for<'a> AsyncFnOnce(Named<'a, A>) -> R) -> R {
    k(Named::new(a)).await
}

/// Async [`name2`].
pub async fn name2_async<A, B, R>(
    a: A,
    b: B,
    k: impl for<'a, 'b> AsyncFnOnce(Named<'a, A>, Named<'b, B>) -> R,
) -> R {
    k(Named::new(a), Named::new(b)).await
}

/// Async [`name3`].
pub async fn name3_async<A, B, C, R>(
    a: A,
    b: B,
    c: C,
    k: impl for<'a, 'b, 'c> AsyncFnOnce(Named<'a, A>, Named<'b, B>, Named<'c, C>) -> R,
) -> R {
    k(Named::new(a), Named::new(b), Named::new(c)).await
}

/// Evidence that a fact holds about some names.
///
/// The trait is sealed: only [`proof!`], [`policy!`], [`And`] and [`Or`]
/// implement it, so a hand-written type cannot pose as a proof in audit logs.
/// Sensitive functions should still demand concrete proof types rather than
/// `impl Proof`: the trait says "this is some proof", not "this proof".
///
/// Implemented by every type declared with [`proof!`] or [`policy!`], and by
/// [`And`] and [`Or`]. Proofs are `Copy` and, for primitive proofs, zero-sized.
pub trait Proof: Copy + __private::Sealed {
    /// The proof's kind, e.g. `"UserIsProjectAdmin"`. Stable across builds,
    /// so it is safe to put in audit logs.
    const KIND: &'static str;

    /// Why access was granted. For a primitive proof this is [`Proof::KIND`];
    /// for a [`policy!`] it is the kind of the proof that satisfied it.
    #[inline(always)]
    fn reason(&self) -> &'static str {
        Self::KIND
    }

    /// Conjunction: evidence that both `self` and `other` hold.
    #[inline(always)]
    fn and<Q: Proof>(self, other: Q) -> And<Self, Q> {
        And(self, other)
    }
}

/// Both `A` and `B` hold.
///
/// Anyone may build an `And` from two proofs they already have, so the
/// fields are public. A function that needs two facts can take
/// `And<IsAdmin<'u, 'p>, PlanIncludes<'p>>`, or simply two parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct And<A, B>(pub A, pub B);

impl<A: Proof, B: Proof> __private::Sealed for And<A, B> {}
impl<A: Proof, B: Proof> Proof for And<A, B> {
    const KIND: &'static str = "And";
}

/// Either `A` or `B` holds: an ad hoc policy, for when a named [`policy!`]
/// would be overkill. [`Proof::reason`] reports which side held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Or<A, B> {
    /// `A` holds.
    Left(A),
    /// `B` holds.
    Right(B),
}

impl<A: Proof, B: Proof> __private::Sealed for Or<A, B> {}
impl<A: Proof, B: Proof> Proof for Or<A, B> {
    const KIND: &'static str = "Or";

    fn reason(&self) -> &'static str {
        match self {
            Or::Left(a) => a.reason(),
            Or::Right(b) => b.reason(),
        }
    }
}

impl<A, B> And<A, B> {
    /// The left conjunct.
    #[inline(always)]
    pub fn left(self) -> A {
        self.0
    }

    /// The right conjunct.
    #[inline(always)]
    pub fn right(self) -> B {
        self.1
    }
}

/// Declares a primitive proof type.
///
/// ```
/// # pub struct UserId; pub struct ProjectId;
/// gdp::proof! {
///     /// The user is an Owner or Member of the project.
///     pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId);
/// }
/// ```
///
/// One lifetime per subject, followed by the type of each subject. This
/// expands to a zero-sized `Copy` struct with a private field, plus two
/// *private* associated functions:
///
/// - `prove(&Named<'u, UserId>, &Named<'p, ProjectId>) -> Self`: mint a proof
///   about these exact named values. Call it after your check passes.
/// - `axiom() -> Self`: mint a proof with no named values at hand. Use it for
///   implications between proofs (an Owner is always an Admin) inside the
///   trusted module, and nowhere else.
///
/// Because both are private, only the module that invokes `proof!` (and its
/// child modules) can create the proof. Put each proof in its own module
/// next to the check it stands for, and keep that module small.
#[macro_export]
macro_rules! proof {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident < $($lt:lifetime),+ $(,)? > ( $($ty:ty),+ $(,)? );
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $Name<$($lt),+> {
            _ghost: ::core::marker::PhantomData<($($crate::Brand<$lt>,)+)>,
        }

        #[allow(dead_code)]
        impl<$($lt),+> $Name<$($lt),+> {
            /// Mints this proof about the given named values. Private to the
            /// trusted module: call it only after the check has passed.
            #[inline(always)]
            fn prove($(_: &$crate::Named<$lt, $ty>),+) -> Self {
                Self { _ghost: ::core::marker::PhantomData }
            }

            /// Mints this proof without named values, for implications
            /// between proofs. Private to the trusted module.
            #[inline(always)]
            fn axiom() -> Self {
                Self { _ghost: ::core::marker::PhantomData }
            }
        }

        impl<$($lt),+> $crate::__private::Sealed for $Name<$($lt),+> {}
        impl<$($lt),+> $crate::Proof for $Name<$($lt),+> {
            const KIND: &'static str = ::core::stringify!($Name);
        }

        impl<$($lt),+> ::core::fmt::Debug for $Name<$($lt),+> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(::core::stringify!($Name))
            }
        }
    };
}

/// Declares a policy: a proof that is satisfied by any one of several proofs.
///
/// ```
/// # pub struct UserId; pub struct ProjectId;
/// # gdp::proof! { pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId); }
/// # gdp::proof! { pub struct UserHasProjectAccess<'u, 'p>(UserId, ProjectId); }
/// gdp::policy! {
///     /// Anyone on the project's team may see the setting.
///     pub enum CanViewProtection<'u, 'p> {
///         Admin(UserIsProjectAdmin<'u, 'p>),
///         Member(UserHasProjectAccess<'u, 'p>),
///     }
/// }
/// ```
///
/// This is the Rust spelling of gdp-ts's union types. It expands to an enum
/// with a `From` impl per variant, so every variant's proof converts with
/// `.into()`, and a [`Proof`] impl whose [`Proof::reason`] reports which proof
/// satisfied it. A policy asserts nothing new, so anyone may build one from
/// a proof they hold; it needs no trusted module.
///
/// Each variant must hold a distinct type (they become `From` impls).
#[macro_export]
macro_rules! policy {
    (@from [$($lt:lifetime),+] $Name:ident;) => {};
    (@from [$($lt:lifetime),+] $Name:ident; $Variant:ident $Inner:ty; $($rest:tt)*) => {
        impl<$($lt),+> ::core::convert::From<$Inner> for $Name<$($lt),+> {
            #[inline(always)]
            fn from(proof: $Inner) -> Self {
                $Name::$Variant(proof)
            }
        }
        $crate::policy!(@from [$($lt),+] $Name; $($rest)*);
    };
    (
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident < $($lt:lifetime),+ $(,)? > {
            $( $Variant:ident ( $Inner:ty ) ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis enum $Name<$($lt),+> {
            $( #[allow(missing_docs)] $Variant($Inner) ),+
        }

        $crate::policy!(@from [$($lt),+] $Name; $($Variant $Inner;)+);

        impl<$($lt),+> $crate::__private::Sealed for $Name<$($lt),+> {}
        impl<$($lt),+> $crate::Proof for $Name<$($lt),+> {
            const KIND: &'static str = ::core::stringify!($Name);

            fn reason(&self) -> &'static str {
                match self {
                    $( $Name::$Variant(p) => $crate::Proof::reason(p) ),+
                }
            }
        }
    };
}

/// Declares that one proof implies another: every `$From` is also a `$To`.
///
/// ```
/// pub struct UserId;
/// pub struct ProjectId;
///
/// mod owner {
///     use super::*;
///     gdp::proof! { pub struct UserIsProjectOwner<'u, 'p>(UserId, ProjectId); }
/// }
///
/// mod admin {
///     use super::*;
///     gdp::proof! { pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId); }
///     // An Owner is always an Admin.
///     gdp::implies!(super::owner::UserIsProjectOwner<'u, 'p> => UserIsProjectAdmin<'u, 'p>);
/// }
/// # fn main() {}
/// ```
///
/// Expands to a `From` impl built on the target's private `axiom()`, so it
/// only compiles inside the target's trusted module: an implication is a
/// claim about the target fact, and only its owner may make it. Use `.into()`
/// at the call site.
#[macro_export]
macro_rules! implies {
    ($From:ty => $To:ident < $($lt:lifetime),+ $(,)? > $(,)?) => {
        impl<$($lt),+> ::core::convert::From<$From> for $To<$($lt),+> {
            #[inline(always)]
            fn from(_: $From) -> Self {
                Self::axiom()
            }
        }
    };
}
