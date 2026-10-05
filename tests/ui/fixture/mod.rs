//! Shared fixture for the compile-fail suite: ids, trusted proof modules, a
//! policy, and sensitive functions that demand proofs.
#![allow(dead_code)]

use gdp::{And, Named};

#[derive(Clone, Copy)]
pub struct UserId(pub u32);
#[derive(Clone, Copy)]
pub struct ProjectId(pub u32);

pub mod admin {
    use super::*;
    gdp::proof! { pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId); }
    pub fn check<'u, 'p>(u: &Named<'u, UserId>, p: &Named<'p, ProjectId>) -> Option<UserIsProjectAdmin<'u, 'p>> {
        (u.0 == 1).then(|| UserIsProjectAdmin::prove(u, p))
    }
}

pub mod access {
    use super::*;
    gdp::proof! { pub struct UserHasProjectAccess<'u, 'p>(UserId, ProjectId); }
    pub fn check<'u, 'p>(u: &Named<'u, UserId>, p: &Named<'p, ProjectId>) -> Option<UserHasProjectAccess<'u, 'p>> {
        Some(UserHasProjectAccess::prove(u, p))
    }
}

pub mod plan {
    use super::*;
    gdp::proof! { pub struct PlanIncludesProtection<'p>(ProjectId); }
    pub fn check<'p>(p: &Named<'p, ProjectId>) -> Option<PlanIncludesProtection<'p>> {
        (p.0 != 0).then(|| PlanIncludesProtection::prove(p))
    }
}

pub use access::UserHasProjectAccess;
pub use admin::UserIsProjectAdmin;
pub use plan::PlanIncludesProtection;

gdp::policy! {
    pub enum CanViewProtection<'u, 'p> {
        Admin(UserIsProjectAdmin<'u, 'p>),
        Team(UserHasProjectAccess<'u, 'p>),
    }
}

pub fn disable_protection<'u, 'p>(_project: &Named<'p, ProjectId>, _proof: UserIsProjectAdmin<'u, 'p>) {}

pub fn set_protection<'u, 'p>(
    _project: &Named<'p, ProjectId>,
    _proof: And<UserIsProjectAdmin<'u, 'p>, PlanIncludesProtection<'p>>,
) {
}

pub fn read_protection<'u, 'p>(_project: &Named<'p, ProjectId>, _proof: impl Into<CanViewProtection<'u, 'p>>) {}

pub fn record_updated_by<'u, 'p>(_user: &Named<'u, UserId>, _project: &Named<'p, ProjectId>, _proof: UserIsProjectAdmin<'u, 'p>) {}
