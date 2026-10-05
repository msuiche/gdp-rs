<img width="115" height="86" style="margin-bottom: -10px" alt="A pixel-art ghost carrying a Rust document" src=".github/assets/logo.png" />

# gdp-rs

**Ghosts of Departed Proofs for Rust.** Authorization and entitlement checks the compiler enforces, with zero runtime cost.

This is a Rust port of [**gdp-ts**](https://github.com/rauchg/gdp-ts) by [**@rauchg**](https://github.com/rauchg). The idea, the framing, the example domain (password protection, plans, policies-as-unions), and the catalogue of mistakes all come from his work; read its README for the full motivation. This crate keeps the design and moves it to a language where the compiler can enforce more of it.

## The problem

```rust
assert_project_admin(user_id, project_id).await?;          // the check
set_password_protection(project_id, password).await;       // the action
```

Nothing connects line 2 to line 1. Delete the check, check a different project, or keep an outdated rule at one call site, and it still compiles. A boolean check forgets *what* it checked.

## The solution

Make the sensitive function demand evidence about its exact arguments:

```rust
pub async fn set_password_protection<'u, 'p>(
    project: &Named<'p, ProjectId>,
    password: &str,
    proof: And<UserIsProjectAdmin<'u, 'p>, PlanIncludesPasswordProtection<'p>>,
)
```

Calling it with no proof, a raw id, a Viewer's proof, a proof about another project, or without the plan check does not compile.

## How it works

**1. Name values.** `name` gives a value a fresh compile-time name, an invariant lifetime that exists only inside the closure. Two `ProjectId`s named separately are incompatible types.

```rust
gdp::name2_async(viewer, project_id, async |user, project| {
    let Some(admin) = user_is_project_admin(&user, &project).await else {
        return Response::Forbidden;
    };
    let Some(plan) = plan_includes_password_protection(&project).await else {
        return Response::PaymentRequired;
    };
    set_password_protection(&project, password, admin.and(plan)).await;
    Response::NoContent
})
.await
```

**2. Prove facts in a trusted module.** `proof!` declares a zero-sized proof type whose constructor is private to the declaring module. That module, and nowhere else, can mint it.

```rust
// proofs/user_is_project_admin.rs
gdp::proof! {
    /// The user is an Owner or Member of the project.
    pub struct UserIsProjectAdmin<'u, 'p>(UserId, ProjectId);
}

pub async fn user_is_project_admin<'u, 'p>(
    user: &Named<'u, UserId>,
    project: &Named<'p, ProjectId>,
) -> Option<UserIsProjectAdmin<'u, 'p>> {
    match db::role_in_project(**user, **project).await? {
        Role::Owner | Role::Member => Some(UserIsProjectAdmin::prove(user, project)),
        Role::Viewer => None,
    }
}
```

**3. Demand proofs.** Sensitive functions take a proof whose lifetimes match their named arguments. The data layer is safe to export: a route, a job or a CLI can call it, and none of them can skip the check.

See [`examples/password_protection.rs`](examples/password_protection.rs) for the complete gdp-ts example, ported (`cargo run --example password_protection`), and [`examples/axum-basic`](examples/axum-basic) for the same thing as an [axum](https://github.com/tokio-rs/axum) service with HTTP-level tests, the counterpart of gdp-ts's `express-basic`. Handler futures stay `Send`, so they run on tokio's multi-threaded runtime.

## Why Rust is a better fit

gdp-ts is upfront that TypeScript is not Haskell: a proof can be forged with `{} as UserIsProjectAdmin<U, P>`, so it relies on a lint preset to catch cheating. Rust closes most of those gaps in the compiler itself.

| Concern | gdp-ts | gdp-rs |
|---|---|---|
| Forging a proof | `{} as Proof<…>` compiles; a lint rule flags it | Private field and private `prove`: a compile error (`E0451`, `E0624`). No lint needed. |
| Minting from the wrong place | `defineProof` is importable anywhere; a lint confines it to `proofs/` | Module privacy: only the module that declares the proof can mint it |
| Escape hatch | `as`, `any` (lint, strict mode) | `unsafe` only, closed in your crate by `#![forbid(unsafe_code)]` (dependencies remain a trust boundary, see [limits](#what-this-does-not-guarantee)) |
| Phantom names | Structural typing ignores unused parameters; needs an invariant `(n: N) => N` slot and `in out` annotations | Lifetimes are nominal; `PhantomData<fn(&'id ()) -> &'id ()>` is invariant by construction |
| Names escaping their scope | TS 5.6+ only, and only via invariance tricks | Higher-ranked `for<'id>` closures: the same rank-2 guarantee as Haskell |
| Runtime cost | A frozen `{ kind }` per proof kind, a frozen `{ value }` per named value | Nothing. Proofs are zero-sized; `Named<T>` has the size of `T` (asserted in tests) |
| Stale evidence | "Keep proofs request-scoped" | Same, plus the borrow checker: evidence about borrowed data blocks mutation of that data (see [justified maps](#justified-containers)) |
| Thread safety | n/a | Proofs and names are `Send + Sync + Copy`, so they cross `.await` and `tokio::spawn` boundaries |

What Rust costs you: names are lifetimes, so mistakes surface as lifetime errors ("lifetime may not live long enough") rather than "proof about the wrong project". The [mistakes catalogue](#the-mistakes-catalogue) pins every one of those messages so you can learn to read them.

## Beyond gdp-ts

- **Async scopes.** `name_async`, `name2_async`, `name3_async` take async closures (`AsyncFnOnce`, Rust 1.85+), so checks and sensitive calls can `.await` while names stay scoped.
- **`policy!`**: the Rust spelling of gdp-ts's proof unions. Generates an enum with a `From` impl per variant, so `admin.into()` satisfies `CanViewProtection`, and a sensitive function can take `impl Into<CanViewProtection<'u, 'p>>`.
- **Audit-ready proofs.** Every proof has a stable `Proof::KIND`, and `proof.reason()` reports which primitive proof satisfied a policy: `audit: read protection on acme because UserHasProjectAccess`. `Proof` is sealed, so a hand-written type cannot pose as a proof in those logs.
- **Conjunction and disjunction.** `admin.and(plan)` builds `And<A, B>`, so a function can demand several facts in one parameter; `Or<A, B>` is an ad hoc policy when a named `policy!` would be overkill.
- **Implications with `implies!`.** A trusted module states "an Owner is always an Admin" once: `gdp::implies!(UserIsProjectOwner<'u, 'p> => UserIsProjectAdmin<'u, 'p>);`, and callers write `owner.into()`. It expands to a `From` impl on the target's private `axiom()`, so only the target's trusted module can declare what implies it.
- **Justified containers.** <a id="justified-containers"></a>A port of Matt Noonan's [`justified-containers`](https://hackage.haskell.org/package/justified-containers), which preceded GDP: `with_map(&map, |m| …)` names a map; `m.member(&k)` returns a `Key` with evidence of membership; `m.get(key)` returns `&V`, not `Option<&V>`. Keys from one map are rejected by another, and the map cannot be mutated (inserted into or removed from) while evidence is alive. Evidence carries the entry it found, so `get` is a field read with no second lookup, and it stays correct even if a key's `Ord`/`Hash` drifts through interior mutability. Works with `BTreeMap` (only `Ord` needed) and `HashMap` (only `Hash + Eq` needed). `with_slice` does the same for indices: `s.index(i)?` checks bounds once, then `s[i]` cannot go out of bounds, and an index of one slice is rejected by another. `s.map(…)` / `s.try_map(…)` build `Aligned` data (adjacency lists, per-element results) under the same name, indexable by the same checked indices: validate once with `try_map`, then traverse without a single `Option`. `cargo run --example justified`.
- **`no_std`.** The core and justified slices need only `core`. Justified maps need `alloc` (`BTreeMap`) or `std` (`HashMap` too).
- **The mistakes catalogue as a test suite.** Every mistake below is a [`trybuild`](https://crates.io/crates/trybuild) compile-fail test with its compiler output pinned, and the honest path is a must-compile test under `#![forbid(unsafe_code)]`.

## The mistakes catalogue

Each of these is a file in [`tests/ui/`](tests/ui/) with a pinned `.stderr`:

| Mistake | Rejected by |
|---|---|
| Proof about project A used for project B | lifetime mismatch |
| Raw `ProjectId` instead of a named one | `E0308` |
| No proof at all | `E0061` |
| Ignoring a failed check (`Option` not handled) | `E0308` |
| Building a proof with a struct literal | `E0451` private field |
| Calling `prove` / `axiom` outside the trusted module | `E0624` private function |
| `unsafe { transmute(()) }` | `forbid(unsafe_code)` |
| A Viewer's proof where an Admin's is required | `E0308` |
| Forgetting the plan (entitlement) check | `E0308` |
| Plan checked for a different project | lifetime mismatch |
| Proof about another user (`updated_by` would lie) | lifetime mismatch |
| The same id named twice and the proofs mixed | `E0521` |
| Returning a name or a proof out of its scope (sync and async) | lifetime mismatch |
| Stashing a proof in a `RefCell` to reuse in a later request | `E0521` |
| Using an outer proof on a value named in an inner scope | `E0521` |
| `mem::swap`-ing two differently named values | `E0521` |
| A callback that asks for two names to be the same | `FnOnce` not general enough |
| Coercing a proof to a different name (variance) | lifetime mismatch |
| `Default::default()` as a proof | `E0277` |
| Implementing `Proof` for a hand-written type | `E0277` sealed |
| Declaring an implication outside the target's trusted module | `E0624` |
| A key of one justified map used on another | `E0521` |
| Mutating a map while holding evidence about its keys | `E0502` borrow conflict |
| An index of one justified slice used on another | `E0521` |
| Aligned data indexed by another slice's index | `E0521` |

## Installation

```toml
[dependencies]
gdp-rs = { git = "https://github.com/msuiche/gdp-rs" }
```

The library is imported as `gdp`. Requires Rust 1.85 or newer (edition 2024, async closures). No dependencies.

Add `#![forbid(unsafe_code)]` to the crates that hold your handlers and data layer. With that, code in those crates can only obtain a proof by passing the check; the remaining way around it is an unsound dependency (see below).

## Recipe

The [gdp-ts recipe](https://github.com/rauchg/gdp-ts/blob/main/skills/gdp-ts/references/recipe.md) carries over almost verbatim:

1. **Brand your ids.** `struct UserId(u64)`, `struct ProjectId(u64)`: newtypes, so a user id never passes for a project id.
2. **One trusted module per fact.** `proof!` plus the function that performs the check, in its own small module. This is the code to review carefully and test.
3. **Policies are enums.** `policy!` over primitive proofs. They assert nothing new, so they need no trust.
4. **Sensitive functions demand proofs** about their exact named arguments.
5. **Name and prove in the handler**, and turn `None` into a 403 / 402.
6. **Forbid `unsafe`** in application crates, instead of gdp-ts's lint preset.

## What this does not guarantee

- **Dependencies are a trust boundary.** `#![forbid(unsafe_code)]` covers your crate, not your dependencies. A dependency with an unsound "safe" API (say, a generic `conjure<T: Copy>()` built on `mem::zeroed`), or an external `macro_rules!` that expands to `unsafe`, can produce any zero-sized value, proofs included. Both were confirmed with probes. This is the same boundary as `unsafeCoerce` in Haskell; audit `unsafe` in your dependency tree (e.g. with `cargo geiger`).
- **Demand concrete proof types.** `Proof` is sealed (behind a `#[doc(hidden)] __private` path, which is greppable but nameable), and `impl Proof` only says "some proof". Sensitive functions should name the exact proof they need.
- **The checks themselves are still code you wrote.** gdp-rs guarantees that the check ran, about the right values, on every path to the sensitive call. It does not guarantee that the check is correct. Test the trusted modules; they are small.
- **Trusted modules include their children.** Rust privacy lets child modules call a parent's private functions. Keep proof modules as leaves.
- **Stale facts about external state.** A proof says the fact held when it was checked. Names are scoped to the request, so proofs are too. Use transactions where a race between check and use matters.
- **`axiom()` is a promise.** It mints a proof without a check. Only use it for implications that are true by definition.

## Prior art and credits

- Guillermo Rauch ([@rauchg](https://github.com/rauchg)), [**gdp-ts**](https://github.com/rauchg/gdp-ts) (2026): the project this ports, including its design, examples and mistakes catalogue.
- Matt Noonan, [Ghosts of Departed Proofs](https://kataskeue.com/gdp.pdf) and [`gdp`](https://hackage.haskell.org/package/gdp) (2018), and [`justified-containers`](https://hackage.haskell.org/package/justified-containers) (2017).
- Ollie Charles, [Who Authorized These Ghosts!?](https://blog.ocharles.org.uk/posts/2019-08-09-who-authorized-these-ghosts.html) (2019).
- Aria Beingessner, [*You Can't Spell Trust Without Rust*](https://faultlore.com/blah/papers/thesis.pdf) (2015), on generativity via invariant lifetimes, and bluss's [`indexing`](https://github.com/bluss/indexing) crate.
- Yanovski, Dang, Jung, Dreyer, [GhostCell](https://plv.mpi-sws.org/rustbelt/ghostcell/) (ICFP 2021), which uses the same branded-lifetime technique.

## License

MIT. See [LICENSE](LICENSE).
