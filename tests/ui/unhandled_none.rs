// `None` is what a failed check returns; it must be handled.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        let proof = admin::check(&user, &project);
        disable_protection(&project, proof);
    });
}
