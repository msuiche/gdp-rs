// An index checked against one slice cannot index another, even a longer one.
fn main() {
    let short = [1];
    let long = [1, 2, 3];
    gdp::justified::with_slice(&long, |long| {
        gdp::justified::with_slice(&short, |short| {
            let i = long.check(2).unwrap();
            short[i];
        })
    });
}
