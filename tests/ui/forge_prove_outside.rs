// `prove` is private to the trusted module.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        disable_protection(&project, UserIsProjectAdmin::prove(&user, &project));
    });
}
