// Forgot the plan check: Password Protection is an entitlement.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        let admin = admin::check(&user, &project).unwrap();
        set_protection(&project, admin);
    });
}
