use axum::{routing::get, Router};

async fn root() -> &'static str { "ok" }
async fn health() -> &'static str { "ok" }

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
}