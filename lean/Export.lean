import GdpPolicy

/-!
Prints `decision` and `grantedBy` for every input as CSV. The output is
committed as `decisions.csv`, and `examples/axum-basic/tests/model.rs`
replays every row against the real service.
-/

open GdpPolicy

def Role.csv : Option Role → String
  | none => "none"
  | some .owner => "owner"
  | some .member => "member"
  | some .viewer => "viewer"

def Plan.csv : Plan → String
  | .hobby => "hobby"
  | .pro => "pro"

def Action.csv : Action → String
  | .view => "view"
  | .set => "set"
  | .disable => "disable"

def Outcome.csv : Outcome → String
  | .allowed => "allowed"
  | .forbidden => "forbidden"
  | .paymentRequired => "payment_required"

def Kind.csv : Kind → String
  | .userIsProjectAdmin => "UserIsProjectAdmin"
  | .userHasProjectAccess => "UserHasProjectAccess"

def main : IO Unit := do
  IO.println "role,plan,action,outcome,granted_by"
  for r in allRoles do
    for p in allPlans do
      for a in allActions do
        let reason := match a, decision r p a with
          | .view, .allowed => (grantedBy r).map Kind.csv |>.getD ""
          | _, _ => ""
        IO.println s!"{Role.csv r},{Plan.csv p},{Action.csv a},{Outcome.csv (decision r p a)},{reason}"
