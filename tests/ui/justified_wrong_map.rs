// A key of one map is not a key of another, even an equal one.
use std::collections::BTreeMap;

fn main() {
    let a = BTreeMap::from([(1, "one")]);
    let b = BTreeMap::from([(1, "one")]);
    gdp::justified::with_map(&a, |a| {
        gdp::justified::with_map(&b, |b| {
            let k = a.member(&1).unwrap();
            b.get(k);
        })
    });
}
