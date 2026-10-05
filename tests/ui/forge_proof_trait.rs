// `Proof` is sealed: a hand-written type cannot pose as a proof.
#[derive(Clone, Copy)]
struct Fake;

impl gdp::Proof for Fake {
    const KIND: &'static str = "UserIsProjectAdmin";
}

fn main() {}
