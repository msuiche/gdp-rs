use gdp::Named;

use crate::db::{Db, Role};
use crate::ids::{ProjectId, UserId};

gdp::proof! {
    /// The user is an Owner or Member of the project.
    pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId);
}

pub async fn user_is_project_admin<'u, 'p>(
    db: &Db,
    user: &Named<'u, UserId>,
    project: &Named<'p, ProjectId>,
) -> Option<UserIsProjectAdmin<'u, 'p>> {
    match db.role_in_project(&user.0, &project.0).await? {
        Role::Owner | Role::Member => Some(UserIsProjectAdmin::prove(user, project)),
        Role::Viewer => None,
    }
}
