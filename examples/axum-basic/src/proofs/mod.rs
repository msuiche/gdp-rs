//! Trusted modules: one per fact, each the only place its proof is minted.
//! Keep them leaves: Rust lets child modules call a parent's private items.

pub mod plan_includes_password_protection;
pub mod protection_policy;
pub mod user_has_project_access;
pub mod user_is_project_admin;
