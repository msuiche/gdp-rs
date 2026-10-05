// `implies!` uses one lifetime list for both sides, so subjects cannot be
// swapped, and proofs with different subjects cannot imply each other.
#[path = "fixture/mod.rs"] mod fixture;

mod admin2 {
    use super::fixture::*;
    gdp::proof! { pub struct Admin2<'u, 'p>(UserId, ProjectId); }
    gdp::implies!(<'u, 'p> super::fixture::PlanIncludesProtection => Admin2);
}

fn main() {}
