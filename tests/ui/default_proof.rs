// Proofs have no `Default`.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name(ProjectId(1), |project| {
        disable_protection(&project, Default::default());
    });
}
