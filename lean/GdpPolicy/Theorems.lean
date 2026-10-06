import GdpPolicy.Model

/-!
# What the policy guarantees

Each theorem is a property of the policy, proved for every role, plan and
action. If someone edits `Model.lean` in a way that breaks one, the build
fails. The differential test then holds the Rust service to the same model.
-/

namespace GdpPolicy

/-- Viewers can see the setting but never change it, whatever the plan. -/
theorem viewer_cannot_change (p : Plan) :
    decision (some .viewer) p .set ≠ .allowed ∧ decision (some .viewer) p .disable ≠ .allowed := by
  cases p <;> decide

/-- Someone off the team is refused everything. -/
theorem stranger_denied (p : Plan) (a : Action) : decision none p a = .forbidden := by
  cases p <;> cases a <;> decide

/-- Turning protection on needs both facts: admin, and a plan that includes it. -/
theorem set_allowed_iff (r : Option Role) (p : Plan) :
    decision r p .set = .allowed ↔ userIsProjectAdmin r = true ∧ planIncludesPasswordProtection p = true := by
  cases r with
  | none => cases p <;> decide
  | some r => cases r <;> cases p <;> decide

/-- Turning protection off never depends on the plan: you can always downgrade. -/
theorem disable_ignores_plan (r : Option Role) : decision r .hobby .disable = decision r .pro .disable := by
  cases r with
  | none => decide
  | some r => cases r <;> decide

/-- Anyone who may change the setting may also see it (`CanManage ⊆ CanView`). -/
theorem manage_implies_view (r : Option Role) (p : Plan) (a : Action) :
    decision r p a = .allowed → decision r p .view = .allowed := by
  cases r with
  | none => cases p <;> cases a <;> decide
  | some r => cases r <;> cases p <;> cases a <;> decide

/-- Every team member can see the setting. -/
theorem team_can_view (r : Role) (p : Plan) : decision (some r) p .view = .allowed := by
  cases r <;> cases p <;> decide

/-- `402 Payment Required` only ever means: an admin, turning protection on,
on a plan without it. It never leaks to users who lack permission anyway. -/
theorem payment_required_only_for_entitled_admins (r : Option Role) (p : Plan) (a : Action) :
    decision r p a = .paymentRequired → a = .set ∧ userIsProjectAdmin r = true ∧ p = .hobby := by
  cases r with
  | none => cases p <;> cases a <;> decide
  | some r => cases r <;> cases p <;> cases a <;> decide

/-- The audit reason for a view is admin exactly when the user is an admin. -/
theorem granted_by_admin_iff (r : Option Role) :
    grantedBy r = some .userIsProjectAdmin ↔ userIsProjectAdmin r = true := by
  cases r with
  | none => decide
  | some r => cases r <;> decide

/-- A successful view always has a reason, and a refused one never does. -/
theorem view_has_reason_iff_allowed (r : Option Role) (p : Plan) :
    (grantedBy r).isSome = true ↔ decision r p .view = .allowed := by
  cases r with
  | none => cases p <;> decide
  | some r => cases r <;> cases p <;> decide

end GdpPolicy
