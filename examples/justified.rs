//! Justified containers: validate once, then traverse without `Option`.
//!
//! Run with `cargo run --example justified`.

use std::collections::HashMap;

use gdp::justified::{Aligned, Index, JSlice, with_slice};

/// A feature flag and the flags it depends on, by name.
struct Flag {
    name: &'static str,
    deps: Vec<&'static str>,
}

/// Depth-first dependency order. Every lookup here is infallible: `deps`
/// holds checked indices, built once by the validation pass in `main`.
fn resolve<'ph>(
    flags: &JSlice<'ph, '_, Flag>,
    deps: &Aligned<'ph, Vec<Index<'ph>>>,
    visited: &mut Aligned<'ph, bool>,
    flag: Index<'ph>,
    out: &mut Vec<&'static str>,
) {
    if visited[flag] {
        return;
    }
    visited[flag] = true;
    for &dep in &deps[flag] {
        resolve(flags, deps, visited, dep, out);
    }
    out.push(flags[flag].name);
}

fn enable_order(all: &[Flag], root: &str) -> Result<Vec<&'static str>, String> {
    with_slice(all, |flags| {
        // The one fallible pass: resolve every name to a checked index.
        let by_name: HashMap<&str, Index> = flags.indices().map(|i| (flags[i].name, i)).collect();
        let lookup = |name: &str| {
            by_name
                .get(name)
                .copied()
                .ok_or(format!("unknown flag {name}"))
        };
        let deps = flags.try_map(|_, f| {
            f.deps
                .iter()
                .map(|d| lookup(d))
                .collect::<Result<Vec<_>, _>>()
        })?;
        let root = lookup(root)?;

        // From here on, nothing can be missing.
        let mut visited = flags.map(|_, _| false);
        let mut out = Vec::new();
        resolve(&flags, &deps, &mut visited, root, &mut out);
        Ok(out)
    })
}

fn main() {
    let mut flags = vec![
        Flag {
            name: "checkout-v2",
            deps: vec!["new-cart", "payments-api"],
        },
        Flag {
            name: "new-cart",
            deps: vec!["payments-api"],
        },
        Flag {
            name: "payments-api",
            deps: vec![],
        },
    ];
    println!("checkout-v2: {:?}", enable_order(&flags, "checkout-v2"));

    // Validation covers the whole graph, so one dangling edge is caught up
    // front, before any traversal starts.
    flags.push(Flag {
        name: "beta-banner",
        deps: vec!["no-such-flag"],
    });
    println!(
        "after adding beta-banner: {:?}",
        enable_order(&flags, "checkout-v2")
    );
}
