// The proof is about project A, not project B.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name3(UserId(1), ProjectId(1), ProjectId(2), |user, a, b| {
        let proof = admin::check(&user, &a).unwrap();
        disable_protection(&b, proof);
    });
}
