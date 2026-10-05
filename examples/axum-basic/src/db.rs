//! Stand-in for a database. Nothing here checks authorization; the data
//! layer in `data.rs` does that by demanding proofs.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Owner,
    Member,
    Viewer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plan {
    Hobby,
    Pro,
}

#[derive(Default)]
struct Tables {
    roles: HashMap<(String, String), Role>,
    plans: HashMap<String, Plan>,
    passwords: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct Db(Arc<RwLock<Tables>>);

impl Db {
    pub async fn seed(&self, roles: &[(&str, &str, Role)], plans: &[(&str, Plan)]) {
        let mut t = self.0.write().await;
        for (user, project, role) in roles {
            t.roles
                .insert((user.to_string(), project.to_string()), *role);
        }
        for (project, plan) in plans {
            t.plans.insert(project.to_string(), *plan);
        }
    }

    pub async fn role_in_project(&self, user: &str, project: &str) -> Option<Role> {
        self.0
            .read()
            .await
            .roles
            .get(&(user.to_owned(), project.to_owned()))
            .copied()
    }

    pub async fn plan_of(&self, project: &str) -> Plan {
        self.0
            .read()
            .await
            .plans
            .get(project)
            .copied()
            .unwrap_or(Plan::Hobby)
    }

    pub async fn write_password(&self, project: &str, password: Option<String>) {
        let mut t = self.0.write().await;
        match password {
            Some(p) => t.passwords.insert(project.to_owned(), p),
            None => t.passwords.remove(project),
        };
    }

    pub async fn has_password(&self, project: &str) -> bool {
        self.0.read().await.passwords.contains_key(project)
    }
}
