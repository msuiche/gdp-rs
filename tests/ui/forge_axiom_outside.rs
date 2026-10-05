// `axiom` is private to the trusted module too.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name(ProjectId(1), |project| {
        disable_protection(&project, UserIsProjectAdmin::axiom());
    });
}
