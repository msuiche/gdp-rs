// The honest paths compile, with no casts and no `unsafe`.
#![forbid(unsafe_code)]
#[path = "../ui/fixture/mod.rs"] mod fixture;
use fixture::*;
use gdp::Proof;

fn main() {
    gdp::name2(UserId(1), ProjectId(1), |user, project| {
        let admin = admin::check(&user, &project).unwrap();
        let plan = plan::check(&project).unwrap();
        // Both facts, about this project and this user.
        set_protection(&project, admin.and(plan));
        // A manage proof is also a view proof; proofs are reusable (Copy).
        read_protection(&project, admin);
        disable_protection(&project, admin);
        record_updated_by(&user, &project, admin);
        // A team proof is a view proof too.
        read_protection(&project, access::check(&user, &project).unwrap());
    });
}
