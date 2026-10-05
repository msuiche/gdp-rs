//! Justified containers: check membership once, then look up without `Option`.
//!
//! Run with `cargo run --example justified`.

use std::collections::HashMap;

use gdp::justified::{JMap, Key, with_map};

/// Resolves a feature-flag dependency graph. Every edge was checked to point
/// at a known flag up front, so the traversal never handles a missing key.
fn resolve<'ph, 'm>(
    flags: &JMap<'ph, 'm, HashMap<&'static str, Vec<&'static str>>>,
    deps: &[Key<'ph, 'm, &'static str, Vec<&'static str>>],
    out: &mut Vec<&'static str>,
) {
    for &dep in deps {
        let name = *dep.key();
        if out.contains(&name) {
            continue;
        }
        // `flags.get(dep)` is infallible: `dep` carries evidence it is a key.
        let children: Vec<_> = flags
            .get(dep)
            .iter()
            .map(|c| flags.member(c).expect("validated"))
            .collect();
        resolve(flags, &children, out);
        out.push(name);
    }
}

fn main() {
    let graph: HashMap<&'static str, Vec<&'static str>> = HashMap::from([
        ("checkout-v2", vec!["new-cart", "payments-api"]),
        ("new-cart", vec!["payments-api"]),
        ("payments-api", vec![]),
    ]);

    let order = with_map(&graph, |flags| {
        // The one fallible pass: validate every edge.
        for key in flags.keys() {
            for child in flags.get(key) {
                flags
                    .member(child)
                    .ok_or(format!("{} depends on unknown flag {child}", key.key()))?;
            }
        }
        let root = flags
            .member("checkout-v2")
            .ok_or("no such flag".to_string())?;
        let mut out = Vec::new();
        resolve(&flags, &[root], &mut out);
        Ok::<_, String>(out)
    });

    println!("enable order: {order:?}");
}
