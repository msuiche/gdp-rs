use gdp::Named;

use crate::db::{Db, Plan};
use crate::ids::ProjectId;

gdp::proof! {
    /// The project's plan includes Password Protection (an entitlement).
    pub struct PlanIncludesPasswordProtection<'p>(ProjectId);
}

pub async fn plan_includes_password_protection<'p>(
    db: &Db,
    project: &Named<'p, ProjectId>,
) -> Option<PlanIncludesPasswordProtection<'p>> {
    (db.plan_of(&project.0).await == Plan::Pro)
        .then(|| PlanIncludesPasswordProtection::prove(project))
}
