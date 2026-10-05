// A callback cannot ask for two names to be the same name.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;
use gdp::Named;

fn body<'x>(_user: Named<'x, UserId>, _project: Named<'x, ProjectId>) {}

fn main() {
    gdp::name2(UserId(1), ProjectId(1), body);
}
