// Same value, named twice: the names still differ.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    let id = ProjectId(1);
    gdp::name2(UserId(1), id, |user, first| {
        let proof = admin::check(&user, &first).unwrap();
        gdp::name(id, |second| disable_protection(&second, proof));
    });
}
