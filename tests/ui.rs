//! The catalogue of mistakes gdp-rs turns into compile errors. Each file in
//! `tests/ui/` has its expected compiler output pinned in a `.stderr` file.
//! Regenerate with `TRYBUILD=overwrite cargo test --test ui`.

#[test]
fn mistakes_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    t.pass("tests/pass/*.rs");
}
