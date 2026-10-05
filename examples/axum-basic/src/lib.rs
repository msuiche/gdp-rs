//! gdp-rs in an axum app: the gdp-ts `express-basic` example, ported.
#![forbid(unsafe_code)]

pub mod data;
pub mod db;
pub mod ids;
pub mod proofs;

use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use gdp::Proof;
use serde::Deserialize;
use serde_json::json;

use crate::data::*;
use crate::db::Db;
use crate::ids::{ProjectId, UserId};
use crate::proofs::plan_includes_password_protection::plan_includes_password_protection;
use crate::proofs::protection_policy::can_view_protection;
use crate::proofs::user_is_project_admin::user_is_project_admin;

/// Errors a handler turns a failed check into.
#[derive(Debug)]
pub enum HttpError {
    Unauthenticated,
    Forbidden(&'static str),
    PaymentRequired(&'static str),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            HttpError::Unauthenticated => (StatusCode::UNAUTHORIZED, "sign in first"),
            HttpError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            HttpError::PaymentRequired(m) => (StatusCode::PAYMENT_REQUIRED, m),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// Authentication only: resolves the signed-in user. Authorization happens in
/// the handler, right where its proof is needed. (A real app reads a session;
/// this example trusts an `x-user` header.)
pub struct Viewer(pub UserId);

impl<S: Send + Sync> FromRequestParts<S> for Viewer {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, HttpError> {
        parts
            .headers
            .get("x-user")
            .and_then(|v| v.to_str().ok())
            .map(|u| Viewer(UserId(u.to_owned())))
            .ok_or(HttpError::Unauthenticated)
    }
}

#[derive(Deserialize)]
pub struct SetProtection {
    pub password: String,
}

async fn get_protection(
    State(db): State<Db>,
    Viewer(viewer): Viewer,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, HttpError> {
    gdp::name2_async(viewer, ProjectId(id), async |user, project| {
        let proof = can_view_protection(&db, &user, &project)
            .await
            .ok_or(HttpError::Forbidden("not on this project's team"))?;
        let (enabled, reason) = is_password_protected(&db, &project, proof).await;
        Ok(Json(json!({ "enabled": enabled, "grantedBy": reason })))
    })
    .await
}

async fn put_protection(
    State(db): State<Db>,
    Viewer(viewer): Viewer,
    Path(id): Path<String>,
    Json(body): Json<SetProtection>,
) -> Result<StatusCode, HttpError> {
    gdp::name2_async(viewer, ProjectId(id), async |user, project| {
        let admin =
            user_is_project_admin(&db, &user, &project)
                .await
                .ok_or(HttpError::Forbidden(
                    "only Owners and Members can change this",
                ))?;
        let plan = plan_includes_password_protection(&db, &project)
            .await
            .ok_or(HttpError::PaymentRequired(
                "Password Protection needs the Pro plan",
            ))?;
        set_password_protection(&db, &project, body.password, admin.and(plan)).await;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

async fn delete_protection(
    State(db): State<Db>,
    Viewer(viewer): Viewer,
    Path(id): Path<String>,
) -> Result<StatusCode, HttpError> {
    gdp::name2_async(viewer, ProjectId(id), async |user, project| {
        let admin =
            user_is_project_admin(&db, &user, &project)
                .await
                .ok_or(HttpError::Forbidden(
                    "only Owners and Members can change this",
                ))?;
        disable_password_protection(&db, &project, admin).await;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

pub fn app(db: Db) -> Router {
    Router::new()
        .route(
            "/projects/{id}/password-protection",
            get(get_protection)
                .put(put_protection)
                .delete(delete_protection),
        )
        .with_state(db)
}
