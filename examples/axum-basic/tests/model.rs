//! Differential test against the Lean model of the policy.
//!
//! `lean/decisions.csv` is generated from `lean/GdpPolicy/Model.lean`, whose
//! properties are proved in `lean/GdpPolicy/Theorems.lean`. Every row is
//! replayed here against the real HTTP service: same role, same plan, same
//! action, and the status code (and audit reason, for views) must match.
//! CI regenerates the CSV from the model and fails if it is stale.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gdp_axum_basic::app;
use gdp_axum_basic::db::{Db, Plan, Role};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

const DECISIONS: &str = include_str!("../../../lean/decisions.csv");

struct Row<'a> {
    role: Option<Role>,
    plan: Plan,
    action: &'a str,
    outcome: &'a str,
    granted_by: &'a str,
    line: &'a str,
}

fn parse(line: &str) -> Row<'_> {
    let cols: Vec<&str> = line.split(',').collect();
    assert_eq!(cols.len(), 5, "malformed row: {line}");
    Row {
        role: match cols[0] {
            "none" => None,
            "owner" => Some(Role::Owner),
            "member" => Some(Role::Member),
            "viewer" => Some(Role::Viewer),
            other => panic!("unknown role {other}"),
        },
        plan: match cols[1] {
            "hobby" => Plan::Hobby,
            "pro" => Plan::Pro,
            other => panic!("unknown plan {other}"),
        },
        action: cols[2],
        outcome: cols[3],
        granted_by: cols[4],
        line,
    }
}

async fn replay(row: &Row<'_>) -> (StatusCode, Value) {
    let db = Db::default();
    let roles: Vec<_> = row
        .role
        .map(|r| ("user", "project", r))
        .into_iter()
        .collect();
    db.seed(&roles, &[("project", row.plan)]).await;

    let (method, body) = match row.action {
        "view" => (Method::GET, Body::empty()),
        "set" => (Method::PUT, Body::from(r#"{"password":"pw"}"#)),
        "disable" => (Method::DELETE, Body::empty()),
        other => panic!("unknown action {other}"),
    };
    let req = Request::builder()
        .method(method)
        .uri("/projects/project/password-protection")
        .header("x-user", "user")
        .header("content-type", "application/json")
        .body(body)
        .unwrap();
    let res = app(db).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn service_matches_the_lean_model() {
    let mut lines = DECISIONS.lines();
    assert_eq!(lines.next(), Some("role,plan,action,outcome,granted_by"));

    let mut checked = 0;
    for line in lines.filter(|l| !l.is_empty()) {
        let row = parse(line);
        let (status, body) = replay(&row).await;
        let outcome = match status {
            StatusCode::OK | StatusCode::NO_CONTENT => "allowed",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::PAYMENT_REQUIRED => "payment_required",
            other => panic!("{}: unexpected status {other}", row.line),
        };
        assert_eq!(
            outcome, row.outcome,
            "model and service disagree on: {}",
            row.line
        );
        if row.action == "view" && row.outcome == "allowed" {
            assert_eq!(
                body["grantedBy"], row.granted_by,
                "audit reason differs on: {}",
                row.line
            );
        }
        checked += 1;
    }
    // 4 memberships x 2 plans x 3 actions: the whole input space.
    assert_eq!(checked, 24);
}
