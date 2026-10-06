/-!
# Password Protection policy, as a Lean model

The policy that `examples/axum-basic` enforces with gdp-rs proofs, stated as
plain functions. gdp-rs guarantees the checks *run*, about the right values,
on every path. This model is where we prove the checks are *right*.

The structure mirrors the Rust code one to one:

| Lean                             | Rust (`examples/axum-basic/src/proofs/`)        |
|----------------------------------|-------------------------------------------------|
| `userIsProjectAdmin`             | `user_is_project_admin` (trusted module)        |
| `userHasProjectAccess`           | `user_has_project_access` (trusted module)      |
| `planIncludesPasswordProtection` | `plan_includes_password_protection` (trusted)   |
| `canViewProtection`              | `CanViewProtection` (`policy!`)                 |
| `decision`                       | the handlers in `src/lib.rs`                    |

`Export.lean` prints `decision` for every input; the axum test suite replays
each row against the real HTTP service (differential testing).
-/

namespace GdpPolicy

/-- A user's role on a project's team. `Option Role` models membership:
`none` is a user who is not on the team at all. -/
inductive Role where
  | owner
  | member
  | viewer
  deriving DecidableEq, Repr

inductive Plan where
  | hobby
  | pro
  deriving DecidableEq, Repr

inductive Action where
  /-- `GET`: see whether protection is on. -/
  | view
  /-- `PUT`: turn protection on (set a password). -/
  | set
  /-- `DELETE`: turn protection off. -/
  | disable
  deriving DecidableEq, Repr

/-- What the service answers. `paymentRequired` is an entitlement failure:
the user may do this, but the project's plan does not include it. -/
inductive Outcome where
  | allowed
  | forbidden
  | paymentRequired
  deriving DecidableEq, Repr

/-- The proof kinds that can grant `view`, for audit logs (`Proof::reason`). -/
inductive Kind where
  | userIsProjectAdmin
  | userHasProjectAccess
  deriving DecidableEq, Repr

/-! ## Primitive facts: one per trusted module -/

def userIsProjectAdmin : Option Role → Bool
  | some .owner | some .member => true
  | _ => false

def userHasProjectAccess (r : Option Role) : Bool :=
  r.isSome

def planIncludesPasswordProtection : Plan → Bool
  | .pro => true
  | .hobby => false

/-! ## Policies: unions of primitive facts -/

/-- Which proof satisfies `CanViewProtection`, if any. Admin is tried first,
matching `can_view_protection`; the order picks the reason, never whether. -/
def canViewProtection (r : Option Role) : Option Kind :=
  if userIsProjectAdmin r then some .userIsProjectAdmin
  else if userHasProjectAccess r then some .userHasProjectAccess
  else none

/-! ## The decision each handler makes -/

def decision (r : Option Role) (p : Plan) : Action → Outcome
  | .view => if (canViewProtection r).isSome then .allowed else .forbidden
  | .set =>
    if !userIsProjectAdmin r then .forbidden
    else if !planIncludesPasswordProtection p then .paymentRequired
    else .allowed
  | .disable => if userIsProjectAdmin r then .allowed else .forbidden

/-- The audit reason a successful `view` reports. -/
def grantedBy (r : Option Role) : Option Kind :=
  canViewProtection r

/-! ## Enumerations, for exhaustive export -/

def allRoles : List (Option Role) := [none, some .owner, some .member, some .viewer]
def allPlans : List Plan := [.hobby, .pro]
def allActions : List Action := [.view, .set, .disable]

end GdpPolicy
