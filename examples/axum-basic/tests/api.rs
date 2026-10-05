use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gdp_axum_basic::app;
use gdp_axum_basic::db::{Db, Plan, Role};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn seeded() -> Db {
    let db = Db::default();
    db.seed(
        &[
            ("alice", "acme", Role::Owner),
            ("dave", "acme", Role::Member),
            ("bob", "acme", Role::Viewer),
            ("carol", "hobby-site", Role::Owner),
        ],
        &[("acme", Plan::Pro), ("hobby-site", Plan::Hobby)],
    )
    .await;
    db
}

async fn call(
    db: &Db,
    method: Method,
    user: Option<&str>,
    project: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(format!("/projects/{project}/password-protection"));
    if let Some(u) = user {
        req = req.header("x-user", u);
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app(db.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn owners_and_members_set_protection_on_pro() {
    let db = seeded().await;
    let pw = Some(json!({ "password": "s3cret" }));
    assert_eq!(
        call(&db, Method::PUT, Some("alice"), "acme", pw.clone())
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&db, Method::PUT, Some("dave"), "acme", pw).await.0,
        StatusCode::NO_CONTENT
    );
    let (status, body) = call(&db, Method::GET, Some("alice"), "acme", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "enabled": true, "grantedBy": "UserIsProjectAdmin" })
    );
}

#[tokio::test]
async fn viewers_can_read_but_not_change() {
    let db = seeded().await;
    let (status, body) = call(
        &db,
        Method::PUT,
        Some("bob"),
        "acme",
        Some(json!({ "password": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "only Owners and Members can change this");
    assert_eq!(
        call(&db, Method::DELETE, Some("bob"), "acme", None).await.0,
        StatusCode::FORBIDDEN
    );
    let (status, body) = call(&db, Method::GET, Some("bob"), "acme", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "enabled": false, "grantedBy": "UserHasProjectAccess" })
    );
}

#[tokio::test]
async fn hobby_plan_needs_upgrade_but_can_always_disable() {
    let db = seeded().await;
    let (status, body) = call(
        &db,
        Method::PUT,
        Some("carol"),
        "hobby-site",
        Some(json!({ "password": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(body["error"], "Password Protection needs the Pro plan");
    assert_eq!(
        call(&db, Method::DELETE, Some("carol"), "hobby-site", None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn strangers_and_anonymous_are_rejected() {
    let db = seeded().await;
    assert_eq!(
        call(&db, Method::GET, Some("mallory"), "acme", None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&db, Method::GET, None, "acme", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    // Alice owns acme, not hobby-site: a proof is about one project.
    assert_eq!(
        call(
            &db,
            Method::PUT,
            Some("alice"),
            "hobby-site",
            Some(json!({ "password": "x" }))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn delete_round_trip() {
    let db = seeded().await;
    call(
        &db,
        Method::PUT,
        Some("alice"),
        "acme",
        Some(json!({ "password": "s3cret" })),
    )
    .await;
    assert_eq!(
        call(&db, Method::DELETE, Some("dave"), "acme", None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&db, Method::GET, Some("bob"), "acme", None).await.1["enabled"],
        false
    );
}
