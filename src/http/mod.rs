use axum::{routing::get, Router};
use crate::AppState;

pub mod auth;

async fn root() -> &'static str { "ok" }
async fn health() -> &'static str { "ok" }

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .merge(auth::routes())
}
