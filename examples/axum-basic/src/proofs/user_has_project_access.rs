use gdp::Named;

use crate::db::Db;
use crate::ids::{ProjectId, UserId};

gdp::proof! {
    /// The user is on the project's team, in any role.
    pub struct UserHasProjectAccess<'u, 'p>(UserId, ProjectId);
}

pub async fn user_has_project_access<'u, 'p>(
    db: &Db,
    user: &Named<'u, UserId>,
    project: &Named<'p, ProjectId>,
) -> Option<UserHasProjectAccess<'u, 'p>> {
    db.role_in_project(&user.0, &project.0)
        .await
        .map(|_| UserHasProjectAccess::prove(user, project))
}
