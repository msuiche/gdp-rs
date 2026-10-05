//! The data layer. Safe to call from any route, job or CLI: every sensitive
//! function demands a proof about its exact arguments.

use gdp::{And, Named, Proof};

use crate::db::Db;
use crate::ids::ProjectId;
use crate::proofs::plan_includes_password_protection::PlanIncludesPasswordProtection;
use crate::proofs::protection_policy::CanViewProtection;
use crate::proofs::user_is_project_admin::UserIsProjectAdmin;

pub async fn set_password_protection<'u, 'p>(
    db: &Db,
    project: &Named<'p, ProjectId>,
    password: String,
    _proof: And<UserIsProjectAdmin<'u, 'p>, PlanIncludesPasswordProtection<'p>>,
) {
    db.write_password(&project.0, Some(password)).await;
}

/// Turning protection off needs no plan: you can always downgrade.
pub async fn disable_password_protection<'u, 'p>(
    db: &Db,
    project: &Named<'p, ProjectId>,
    _proof: UserIsProjectAdmin<'u, 'p>,
) {
    db.write_password(&project.0, None).await;
}

pub async fn is_password_protected<'u, 'p>(
    db: &Db,
    project: &Named<'p, ProjectId>,
    proof: impl Into<CanViewProtection<'u, 'p>>,
) -> (bool, &'static str) {
    (db.has_password(&project.0).await, proof.into().reason())
}
