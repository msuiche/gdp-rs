// A proof from an outer scope does not apply to a value named in an inner one.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        let proof = admin::check(&user, &project).unwrap();
        gdp::name(ProjectId(2), |other| disable_protection(&other, proof));
    });
}
