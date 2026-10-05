// Stashing a proof in outer state to reuse it in a later request.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;
use std::cell::RefCell;

fn main() {
    let stash = RefCell::new(None);
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        *stash.borrow_mut() = admin::check(&user, &project);
    });
    gdp::name(ProjectId(2), |project| {
        disable_protection(&project, stash.borrow().unwrap());
    });
}
