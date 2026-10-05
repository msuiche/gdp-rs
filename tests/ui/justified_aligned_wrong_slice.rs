// Data aligned with one slice cannot be indexed by another slice's index.
fn main() {
    let a = [1, 2, 3];
    let b = [1, 2, 3];
    gdp::justified::with_slice(&a, |a| {
        let doubled = a.map(|_, x| x * 2);
        gdp::justified::with_slice(&b, |b| {
            let i = b.index(0).unwrap();
            doubled[i];
        })
    });
}
