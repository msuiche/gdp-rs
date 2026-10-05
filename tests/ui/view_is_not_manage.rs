// Viewing is not managing: a team member's proof cannot change the setting.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        let proof = access::check(&user, &project).unwrap();
        disable_protection(&project, proof);
    });
}
