# Lean model of the policy

gdp-rs guarantees that the authorization checks **run**, about the right
values, on every path. It cannot tell you whether the checks are **right**.
This directory closes that gap the way AWS's Cedar does: a small Lean model
of the policy, proved properties about it, and a differential test that holds
the real service to the model.

```
GdpPolicy/Model.lean     the policy: primitive facts, the CanViewProtection policy, decision
GdpPolicy/Theorems.lean  what it guarantees, proved for every role, plan and action
Export.lean              prints decision + audit reason for every input
decisions.csv            that output, committed
```

`examples/axum-basic/tests/model.rs` replays every row of `decisions.csv`
against the real HTTP service and requires the same status code and, for
views, the same audit reason (`grantedBy`).

## What is proved

| Theorem | Meaning |
|---|---|
| `viewer_cannot_change` | Viewers can never set or disable protection, on any plan |
| `stranger_denied` | Someone off the team is refused everything |
| `set_allowed_iff` | Turning protection on is allowed exactly for admins on a plan that includes it |
| `disable_ignores_plan` | Turning protection off never depends on the plan |
| `manage_implies_view` | Anyone allowed any action may also view (`CanManage ⊆ CanView`) |
| `team_can_view` | Every team member can view |
| `payment_required_only_for_entitled_admins` | `402` only ever means an admin turning protection on, on Hobby |
| `granted_by_admin_iff` | The audit reason is `UserIsProjectAdmin` exactly for admins |
| `view_has_reason_iff_allowed` | A successful view always has an audit reason, a refused one never |

## Workflow

```bash
lake build                                 # checks every proof
lake exe export > decisions.csv            # regenerate after changing the model
cd ../examples/axum-basic && cargo test --test model   # service vs model
```

CI runs all three and fails if `decisions.csv` is stale. A change to the
policy therefore has to land in three places that check each other: the
model (and its proofs), the table, and the Rust trusted modules.

## The chain of guarantees

1. **Lean** proves the policy has the properties above.
2. **The differential test** shows the Rust service implements that policy
   for every input (the input space here is 24 cases, so it is exhaustive).
3. **gdp-rs** guarantees the data layer cannot be reached without the checks
   the service makes, about the exact user and project in the request.

What stays on trust: that the model's properties are the ones you want, and
the HTTP and database layers beneath the checks.
