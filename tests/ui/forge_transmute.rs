// The last escape hatch, `unsafe`, is closed by `#![forbid(unsafe_code)]`.
#![forbid(unsafe_code)]
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn main() {
    gdp::name(ProjectId(1), |project| {
        let forged: UserIsProjectAdmin<'_, '_> = unsafe { core::mem::transmute(()) };
        disable_protection(&project, forged);
    });
}
