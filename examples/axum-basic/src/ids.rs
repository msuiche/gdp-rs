//! Branded ids: a `UserId` never passes for a `ProjectId`.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectId(pub String);
