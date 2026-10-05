// Only the target proof's trusted module may declare what implies it.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

gdp::implies!(UserHasProjectAccess<'u, 'p> => UserIsProjectAdmin<'u, 'p>);

fn main() {}
