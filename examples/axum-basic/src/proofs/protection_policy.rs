//! Who may see and change Password Protection. Policies assert nothing new,
//! so this module mints nothing and needs no trust.

use gdp::Named;

use super::user_has_project_access::{UserHasProjectAccess, user_has_project_access};
use super::user_is_project_admin::{UserIsProjectAdmin, user_is_project_admin};
use crate::db::Db;
use crate::ids::{ProjectId, UserId};

gdp::policy! {
    /// Anyone on the team may see whether protection is on.
    pub enum CanViewProtection<'u, 'p> {
        Admin(UserIsProjectAdmin<'u, 'p>),
        Team(UserHasProjectAccess<'u, 'p>),
    }
}

pub async fn can_view_protection<'u, 'p>(
    db: &Db,
    user: &Named<'u, UserId>,
    project: &Named<'p, ProjectId>,
) -> Option<CanViewProtection<'u, 'p>> {
    // Order decides *which* proof you get (useful for audit logs), never *whether*.
    if let Some(admin) = user_is_project_admin(db, user, project).await {
        return Some(admin.into());
    }
    user_has_project_access(db, user, project)
        .await
        .map(Into::into)
}
