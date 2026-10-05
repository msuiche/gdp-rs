// A raw id is not a named value; name it first.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        let proof = admin::check(&user, &project).unwrap();
        disable_protection(&ProjectId(1), proof);
    });
}
