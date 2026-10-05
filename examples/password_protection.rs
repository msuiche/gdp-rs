//! The gdp-ts "basic" example, in Rust: who may see and change a project's
//! Password Protection setting, enforced by the type checker.
//!
//! Run with `cargo run --example password_protection`.

use std::cell::RefCell;
use std::collections::HashMap;

/// Branded ids: a `UserId` never passes for a `ProjectId`.
mod ids {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct UserId(pub &'static str);
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ProjectId(pub &'static str);
}

/// Stand-in for a database. Nothing here checks authorization; that is the point.
mod db {
    use super::*;
    use ids::*;

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Role {
        Owner,
        Member,
        Viewer,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Plan {
        Hobby,
        Pro,
    }

    thread_local! {
        static ROLES: HashMap<(&'static str, &'static str), Role> = HashMap::from([
            (("alice", "acme"), Role::Owner),
            (("bob", "acme"), Role::Viewer),
            (("dave", "acme"), Role::Member),
            (("carol", "hobby-site"), Role::Owner),
        ]);
        static PLANS: HashMap<&'static str, Plan> =
            HashMap::from([("acme", Plan::Pro), ("hobby-site", Plan::Hobby)]);
        pub static PASSWORDS: RefCell<HashMap<&'static str, String>> = RefCell::new(HashMap::new());
    }

    pub async fn role_in_project(user: UserId, project: ProjectId) -> Option<Role> {
        ROLES.with(|r| r.get(&(user.0, project.0)).copied())
    }

    pub async fn plan_of(project: ProjectId) -> Plan {
        PLANS.with(|p| p.get(project.0).copied().unwrap_or(Plan::Hobby))
    }
}

/// Trusted modules: one per fact. Each is the only place its proof can be minted.
mod proofs {
    pub mod user_is_project_admin {
        use crate::db::{self, Role};
        use crate::ids::{ProjectId, UserId};
        use gdp::Named;

        gdp::proof! {
            /// The user is an Owner or Member of the project.
            pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId);
        }

        pub async fn user_is_project_admin<'u, 'p>(
            user: &Named<'u, UserId>,
            project: &Named<'p, ProjectId>,
        ) -> Option<UserIsProjectAdmin<'u, 'p>> {
            match db::role_in_project(**user, **project).await? {
                Role::Owner | Role::Member => Some(UserIsProjectAdmin::prove(user, project)),
                Role::Viewer => None,
            }
        }
    }

    pub mod user_has_project_access {
        use crate::db;
        use crate::ids::{ProjectId, UserId};
        use gdp::Named;

        gdp::proof! {
            /// The user is on the project's team, in any role.
            pub struct UserHasProjectAccess<'u, 'p>(UserId, ProjectId);
        }

        pub async fn user_has_project_access<'u, 'p>(
            user: &Named<'u, UserId>,
            project: &Named<'p, ProjectId>,
        ) -> Option<UserHasProjectAccess<'u, 'p>> {
            db::role_in_project(**user, **project)
                .await
                .map(|_| UserHasProjectAccess::prove(user, project))
        }
    }

    pub mod plan_includes_password_protection {
        use crate::db::{self, Plan};
        use crate::ids::ProjectId;
        use gdp::Named;

        gdp::proof! {
            /// The project's plan includes Password Protection (an entitlement).
            pub struct PlanIncludesPasswordProtection<'p>(ProjectId);
        }

        pub async fn plan_includes_password_protection<'p>(
            project: &Named<'p, ProjectId>,
        ) -> Option<PlanIncludesPasswordProtection<'p>> {
            (db::plan_of(**project).await == Plan::Pro)
                .then(|| PlanIncludesPasswordProtection::prove(project))
        }
    }

    /// Policies assert nothing new, so this module is not trusted and mints nothing.
    pub mod protection_policy {
        use super::user_has_project_access::*;
        use super::user_is_project_admin::*;
        use crate::ids::{ProjectId, UserId};
        use gdp::Named;

        gdp::policy! {
            /// Anyone on the team may see whether protection is on.
            pub enum CanViewProtection<'u, 'p> {
                Admin(UserIsProjectAdmin<'u, 'p>),
                Team(UserHasProjectAccess<'u, 'p>),
            }
        }

        pub async fn can_view_protection<'u, 'p>(
            user: &Named<'u, UserId>,
            project: &Named<'p, ProjectId>,
        ) -> Option<CanViewProtection<'u, 'p>> {
            // Order decides *which* proof you get (useful for audit logs), never *whether*.
            if let Some(admin) = user_is_project_admin(user, project).await {
                return Some(admin.into());
            }
            user_has_project_access(user, project).await.map(Into::into)
        }
    }
}

/// The data layer: safe to export, because every sensitive function demands proof.
mod data {
    use crate::db::PASSWORDS;
    use crate::ids::ProjectId;
    use crate::proofs::plan_includes_password_protection::PlanIncludesPasswordProtection;
    use crate::proofs::protection_policy::CanViewProtection;
    use crate::proofs::user_is_project_admin::UserIsProjectAdmin;
    use gdp::{And, Named, Proof};

    pub async fn set_password_protection<'u, 'p>(
        project: &Named<'p, ProjectId>,
        password: &str,
        proof: And<UserIsProjectAdmin<'u, 'p>, PlanIncludesPasswordProtection<'p>>,
    ) {
        println!("  audit: set protection on {} ({:?})", project.0, proof);
        PASSWORDS.with(|p| p.borrow_mut().insert(project.0, password.to_owned()));
    }

    pub async fn is_password_protected<'u, 'p>(
        project: &Named<'p, ProjectId>,
        proof: impl Into<CanViewProtection<'u, 'p>>,
    ) -> bool {
        let proof = proof.into();
        println!(
            "  audit: read protection on {} because {}",
            project.0,
            proof.reason()
        );
        PASSWORDS.with(|p| p.borrow().contains_key(project.0))
    }
}

/// Handlers: name the inputs, prove, turn `None` into a response.
mod handlers {
    use crate::data::*;
    use crate::ids::*;
    use crate::proofs::plan_includes_password_protection::*;
    use crate::proofs::protection_policy::*;
    use crate::proofs::user_is_project_admin::*;
    use gdp::Proof;

    #[derive(Debug, PartialEq)]
    pub enum Response {
        NoContent,
        Ok(bool),
        Forbidden(&'static str),
        PaymentRequired,
    }

    pub async fn put_password_protection(
        viewer: UserId,
        project: ProjectId,
        password: &str,
    ) -> Response {
        gdp::name2_async(viewer, project, async |user, project| {
            let Some(admin) = user_is_project_admin(&user, &project).await else {
                return Response::Forbidden("only Owners and Members can change this");
            };
            let Some(plan) = plan_includes_password_protection(&project).await else {
                return Response::PaymentRequired;
            };
            set_password_protection(&project, password, admin.and(plan)).await;
            Response::NoContent
        })
        .await
    }

    pub async fn get_password_protection(viewer: UserId, project: ProjectId) -> Response {
        gdp::name2_async(
            viewer,
            project,
            async |user, project| match can_view_protection(&user, &project).await {
                Some(proof) => Response::Ok(is_password_protected(&project, proof).await),
                None => Response::Forbidden("not on this project's team"),
            },
        )
        .await
    }
}

/// A dependency-free executor; every future in this example is ready immediately.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

fn main() {
    use handlers::*;
    use ids::*;

    let cases = [
        (
            "alice (Owner) sets on acme (Pro)",
            put_password_protection(UserId("alice"), ProjectId("acme"), "s3cret"),
        ),
        (
            "dave (Member) sets on acme",
            put_password_protection(UserId("dave"), ProjectId("acme"), "s3cret2"),
        ),
        (
            "bob (Viewer) sets on acme",
            put_password_protection(UserId("bob"), ProjectId("acme"), "pwned"),
        ),
        (
            "carol (Owner) sets on hobby-site (Hobby)",
            put_password_protection(UserId("carol"), ProjectId("hobby-site"), "x"),
        ),
        (
            "mallory (stranger) sets on acme",
            put_password_protection(UserId("mallory"), ProjectId("acme"), "pwned"),
        ),
    ];
    for (label, fut) in cases {
        println!("{label}");
        println!("  -> {:?}", block_on(fut));
    }

    for (label, fut) in [
        (
            "bob (Viewer) reads acme",
            get_password_protection(UserId("bob"), ProjectId("acme")),
        ),
        (
            "alice (Owner) reads acme",
            get_password_protection(UserId("alice"), ProjectId("acme")),
        ),
        (
            "mallory reads acme",
            get_password_protection(UserId("mallory"), ProjectId("acme")),
        ),
    ] {
        println!("{label}");
        println!("  -> {:?}", block_on(fut));
    }

    println!(
        "proof size: {} bytes",
        std::mem::size_of::<proofs::user_is_project_admin::UserIsProjectAdmin<'static, 'static>>()
    );
}
