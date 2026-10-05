// `mem::swap` cannot exchange values that carry different names.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, mut project| {
        let proof = admin::check(&user, &project).unwrap();
        gdp::name(ProjectId(2), |mut other| {
            std::mem::swap(&mut project, &mut other);
            disable_protection(&project, proof);
        });
    });
}
