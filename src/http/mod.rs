use axum::{routing::get, Router};
use crate::AppState;

pub mod auth;
pub mod guards;

async fn root() -> &'static str { "ok" }
async fn health() -> &'static str { "ok" }

async fn admin_ping(sess: guards::AuthSession) -> Result<&'static str, axum::http::StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"])?;
    Ok("pong")
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/protected/admin-ping", get(admin_ping))
        .merge(auth::routes())
}
