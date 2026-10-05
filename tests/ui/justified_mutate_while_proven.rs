// Evidence cannot go stale: the map is borrowed while keys are alive.
use std::collections::BTreeMap;

fn main() {
    let mut m = BTreeMap::from([(1, "one")]);
    gdp::justified::with_map(&m, |jm| {
        let k = jm.member(&1).unwrap();
        m.remove(&1);
        jm.get(k);
    });
}
