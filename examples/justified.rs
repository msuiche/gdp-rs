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

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    New,
    InProgress,
    Done,
}

/// Depth-first dependency order. Every lookup here is infallible: `deps`
/// holds checked indices, built once by the validation pass. The only
/// possible failure is a property of the graph itself: a cycle.
fn resolve<'ph>(
    flags: &JSlice<'ph, '_, Flag>,
    deps: &Aligned<'ph, Vec<Index<'ph>>>,
    marks: &mut Aligned<'ph, Mark>,
    flag: Index<'ph>,
    out: &mut Vec<&'static str>,
) -> Result<(), String> {
    match marks[flag] {
        Mark::Done => return Ok(()),
        Mark::InProgress => return Err(format!("dependency cycle through {}", flags[flag].name)),
        Mark::New => {}
    }
    marks[flag] = Mark::InProgress;
    for &dep in &deps[flag] {
        resolve(flags, deps, marks, dep, out)?;
    }
    marks[flag] = Mark::Done;
    out.push(flags[flag].name);
    Ok(())
}

fn enable_order(all: &[Flag], root: &str) -> Result<Vec<&'static str>, String> {
    with_slice(all, |flags| {
        // The one fallible pass: resolve every name to a checked index.
        let mut by_name: HashMap<&str, Index> = HashMap::new();
        for (i, flag) in flags.enumerate() {
            if by_name.insert(flag.name, i).is_some() {
                return Err(format!("duplicate flag {}", flag.name));
            }
        }
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
        let mut marks = flags.map(|_, _| Mark::New);
        let mut out = Vec::new();
        resolve(&flags, &deps, &mut marks, root, &mut out)?;
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
        "with a dangling edge: {:?}",
        enable_order(&flags, "checkout-v2")
    );

    flags.pop();
    flags[2].deps.push("checkout-v2");
    println!("with a cycle: {:?}", enable_order(&flags, "checkout-v2"));

    flags[2].deps.pop();
    flags.push(Flag {
        name: "new-cart",
        deps: vec![],
    });
    println!(
        "with a duplicate: {:?}",
        enable_order(&flags, "checkout-v2")
    );
}
