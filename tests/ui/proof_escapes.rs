// Proofs cannot leave their scope either: they would outlive the request.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    let _stash = gdp::name2(UserId(1), ProjectId(1), |user, project| admin::check(&user, &project));
}
