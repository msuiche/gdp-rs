// Proofs are invariant: a proof about one name cannot be coerced to another.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

fn widen<'a, 'b, 'p>(proof: UserIsProjectAdmin<'a, 'p>) -> UserIsProjectAdmin<'b, 'p>
where
    'a: 'b,
{
    proof
}

fn main() {}
