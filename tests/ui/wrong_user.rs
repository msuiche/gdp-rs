// The proof is about another user; `updated_by` would name the wrong person.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name3(UserId(1), UserId(2), ProjectId(1), |alice, bob, project| {
        let proof = admin::check(&alice, &project).unwrap();
        record_updated_by(&bob, &project, proof);
    });
}
