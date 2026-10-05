// The plan check was for project B.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;
use gdp::Proof;

fn main() {
    gdp::name3(UserId(1), ProjectId(1), ProjectId(2), |user, a, b| {
        let admin = admin::check(&user, &a).unwrap();
        let plan = plan::check(&b).unwrap();
        set_protection(&a, admin.and(plan));
    });
}
