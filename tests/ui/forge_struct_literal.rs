// You cannot build a proof by hand: its field is private to the trusted module.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name(ProjectId(1), |project| {
        disable_protection(&project, UserIsProjectAdmin { _ghost: core::marker::PhantomData });
    });
}
