use core::mem::size_of;
use std::collections::{BTreeMap, HashMap};

use gdp::{And, Named, Proof};

#[derive(Clone, Copy, Debug, PartialEq)]
struct UserId(u64);
#[derive(Clone, Copy, Debug, PartialEq)]
struct ProjectId(u64);

mod owner {
    use super::*;
    gdp::proof! {
        /// Owners can do anything.
        pub struct UserIsProjectOwner<'u, 'p>(UserId, ProjectId);
    }
    pub fn check<'u, 'p>(
        u: &Named<'u, UserId>,
        p: &Named<'p, ProjectId>,
    ) -> Option<UserIsProjectOwner<'u, 'p>> {
        (u.0 == 1).then(|| UserIsProjectOwner::prove(u, p))
    }
}

mod admin {
    use super::owner::UserIsProjectOwner;
    use super::*;
    gdp::proof! { pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId); }
    pub fn check<'u, 'p>(
        u: &Named<'u, UserId>,
        p: &Named<'p, ProjectId>,
    ) -> Option<UserIsProjectAdmin<'u, 'p>> {
        (u.0 <= 2).then(|| UserIsProjectAdmin::prove(u, p))
    }
    // An implication, stated once in the trusted module with `axiom`.
    impl<'u, 'p> From<UserIsProjectOwner<'u, 'p>> for UserIsProjectAdmin<'u, 'p> {
        fn from(_: UserIsProjectOwner<'u, 'p>) -> Self {
            Self::axiom()
        }
    }
}

mod team {
    use super::*;
    gdp::proof! { pub struct OnTeam<'u, 'p>(UserId, ProjectId); }
    pub fn check<'u, 'p>(
        u: &Named<'u, UserId>,
        p: &Named<'p, ProjectId>,
    ) -> Option<OnTeam<'u, 'p>> {
        (u.0 <= 3).then(|| OnTeam::prove(u, p))
    }
}

use admin::UserIsProjectAdmin;
use owner::UserIsProjectOwner;
use team::OnTeam;

gdp::policy! {
    pub enum CanView<'u, 'p> {
        Admin(UserIsProjectAdmin<'u, 'p>),
        Team(OnTeam<'u, 'p>),
    }
}

fn can_view<'u, 'p>(u: &Named<'u, UserId>, p: &Named<'p, ProjectId>) -> Option<CanView<'u, 'p>> {
    admin::check(u, p)
        .map(Into::into)
        .or_else(|| team::check(u, p).map(Into::into))
}

fn admin_only<'u, 'p>(p: &Named<'p, ProjectId>, _: UserIsProjectAdmin<'u, 'p>) -> u64 {
    p.0
}

fn block_on<F: core::future::Future>(f: F) -> F::Output {
    let mut f = core::pin::pin!(f);
    let mut cx = core::task::Context::from_waker(core::task::Waker::noop());
    loop {
        if let core::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

#[test]
fn proofs_are_zero_sized_and_names_are_free() {
    assert_eq!(size_of::<UserIsProjectAdmin<'static, 'static>>(), 0);
    assert_eq!(
        size_of::<And<UserIsProjectAdmin<'static, 'static>, OnTeam<'static, 'static>>>(),
        0
    );
    assert_eq!(size_of::<CanView<'static, 'static>>(), 1); // a discriminant, nothing else
    assert_eq!(size_of::<Named<'static, u64>>(), size_of::<u64>());
    assert_eq!(size_of::<Option<UserIsProjectAdmin<'static, 'static>>>(), 1);
}

#[test]
fn proofs_and_names_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<UserIsProjectAdmin<'static, 'static>>();
    assert_send_sync::<CanView<'static, 'static>>();
    assert_send_sync::<Named<'static, u64>>();
}

#[test]
fn checks_gate_the_sensitive_call() {
    let run = |user| {
        gdp::name2(UserId(user), ProjectId(42), |u, p| {
            admin::check(&u, &p).map(|proof| admin_only(&p, proof))
        })
    };
    assert_eq!(run(1), Some(42));
    assert_eq!(run(2), Some(42));
    assert_eq!(run(3), None);
}

#[test]
fn kinds_and_reasons_are_reportable() {
    gdp::name2(UserId(3), ProjectId(1), |u, p| {
        let view = can_view(&u, &p).unwrap();
        assert_eq!(view.reason(), "OnTeam");
        assert_eq!(<CanView as Proof>::KIND, "CanView");
        assert_eq!(format!("{:?}", view), "Team(OnTeam)");
    });
    gdp::name2(UserId(2), ProjectId(1), |u, p| {
        assert_eq!(can_view(&u, &p).unwrap().reason(), "UserIsProjectAdmin");
        let both = admin::check(&u, &p)
            .unwrap()
            .and(team::check(&u, &p).unwrap());
        assert_eq!(both.reason(), "And");
        assert_eq!(format!("{:?}", both), "And(UserIsProjectAdmin, OnTeam)");
    });
    gdp::name2(UserId(4), ProjectId(1), |u, p| {
        assert!(can_view(&u, &p).is_none())
    });
}

#[test]
fn implications_via_axiom() {
    gdp::name2(UserId(1), ProjectId(7), |u, p| {
        let owner: UserIsProjectOwner = owner::check(&u, &p).unwrap();
        assert_eq!(admin_only(&p, owner.into()), 7);
    });
}

#[test]
fn async_scopes() {
    async fn check<'u, 'p>(
        u: &Named<'u, UserId>,
        p: &Named<'p, ProjectId>,
    ) -> Option<UserIsProjectAdmin<'u, 'p>> {
        core::future::ready(()).await;
        admin::check(u, p)
    }
    let got = block_on(gdp::name2_async(UserId(1), ProjectId(9), async |u, p| {
        let proof = check(&u, &p).await?;
        Some(admin_only(&p, proof))
    }));
    assert_eq!(got, Some(9));
    let n = block_on(gdp::name3_async(1u8, 2u8, 3u8, async |a, b, c| {
        *a + *b + *c
    }));
    assert_eq!(n, 6);
    assert_eq!(block_on(gdp::name_async(5u8, async |a| a.into_inner())), 5);
}

#[test]
fn named_accessors() {
    gdp::name(String::from("acme"), |n| {
        assert_eq!(n.value(), "acme");
        assert_eq!(n.len(), 4); // Deref
        assert_eq!(n.as_ref(), "acme");
        assert_eq!(format!("{n:?}"), "Named(\"acme\")");
        assert_eq!(n.clone().into_inner(), "acme");
    });
}

#[test]
fn justified_maps() {
    let b = BTreeMap::from([("http", 80), ("https", 443)]);
    gdp::justified::with_map(&b, |m| {
        assert_eq!(m.len(), 2);
        assert!(!m.is_empty());
        let k = m.member("https").unwrap();
        assert_eq!(*m.get(k), 443);
        assert_eq!(*k.key(), "https");
        assert!(m.member("gopher").is_none());
        let sum: i32 = m.keys().map(|k| *m.get(k)).sum();
        assert_eq!(sum, 523);
    });
    let h: HashMap<String, u8> = HashMap::from([("a".to_string(), 1)]);
    gdp::justified::with_map(&h, |m| {
        let k = m.member("a").unwrap(); // `Borrow<str>` lookups
        assert_eq!(*m.get(k), 1);
        assert_eq!(format!("{k:?}"), "Key(\"a\")");
    });
}

// Regression: `member` used to demand `Ord + Hash` of every key type.
#[test]
fn justified_member_needs_only_the_maps_own_bounds() {
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct OrdOnly(u32);
    #[derive(PartialEq, Eq, Hash)]
    struct HashOnly(u32);

    let b = BTreeMap::from([(OrdOnly(1), "b")]);
    gdp::justified::with_map(&b, |m| {
        assert_eq!(*m.get(m.member(&OrdOnly(1)).unwrap()), "b")
    });
    let h = HashMap::from([(HashOnly(1), "h")]);
    gdp::justified::with_map(&h, |m| {
        assert_eq!(*m.get(m.member(&HashOnly(1)).unwrap()), "h")
    });
}

// Regression: `get` used to re-look-up the key and could panic when a key's
// ordering changed through interior mutability. Evidence now carries the entry.
#[test]
#[allow(clippy::mutable_key_type)] // the point of the test
fn justified_get_survives_interior_mutability() {
    use std::cell::Cell;
    let m = BTreeMap::from([(Cell::new(1u32), "one"), (Cell::new(5u32), "five")]);
    gdp::justified::with_map(&m, |jm| {
        let k = jm.keys().nth(1).unwrap();
        k.key().set(0);
        assert_eq!(*jm.get(k), "five");
    });
}

#[test]
fn justified_get_survives_a_drifting_hasher() {
    use std::cell::Cell;
    use std::hash::{BuildHasher, DefaultHasher, Hasher};
    #[derive(Default)]
    struct Drifting(Cell<u64>);
    impl BuildHasher for Drifting {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> DefaultHasher {
            let mut h = DefaultHasher::new();
            h.write_u64(self.0.get());
            h
        }
    }
    let mut m: HashMap<u32, &str, Drifting> = HashMap::default();
    m.insert(1, "one");
    gdp::justified::with_map(&m, |jm| {
        let k = jm.member(&1).unwrap();
        m.hasher().0.set(7);
        assert_eq!(*jm.get(k), "one");
    });
}

#[test]
fn justified_slices() {
    let v = [10, 20, 30];
    gdp::justified::with_slice(&v, |s| {
        assert_eq!(s.len(), 3);
        let i = s.index(1).unwrap();
        assert_eq!(s[i], 20);
        assert_eq!(*s.get(i), 20);
        assert_eq!(i.get(), 1);
        assert!(s.index(3).is_none());
        assert_eq!(s[s.first().unwrap()], 10);
        assert_eq!(s[s.last().unwrap()], 30);
        assert_eq!(s.next(s.last().unwrap()), None);
        assert_eq!(s.prev(s.first().unwrap()), None);
        assert_eq!(s.next(i).map(|j| s[j]), Some(30));
        assert_eq!(
            s.indices().rev().map(|i| s[i]).collect::<Vec<_>>(),
            [30, 20, 10]
        );
        assert_eq!(format!("{i:?}"), "Index(1)");
    });
    let empty: [u8; 0] = [];
    gdp::justified::with_slice(&empty, |s| {
        assert!(s.is_empty());
        assert!(s.first().is_none() && s.last().is_none());
        assert_eq!(s.indices().len(), 0);
    });
}
