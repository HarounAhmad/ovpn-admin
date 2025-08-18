use axum::{middleware, routing::get, Router};
use tower_http::set_header::SetResponseHeaderLayer;
use axum::http::HeaderValue;
use crate::AppState;

pub mod auth;
pub mod guards;
pub mod csrf;

async fn root() -> &'static str { "ok" }
async fn health() -> &'static str { "ok" }

async fn admin_ping(sess: guards::AuthSession) -> Result<&'static str, axum::http::StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"])?;
    Ok("pong")
}

pub fn router() -> Router<AppState> {
    let sec_headers = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'none'; form-action 'self'"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .into_inner();

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/protected/admin-ping", get(admin_ping))
        .route("/auth/csrf", get(csrf::issue_token))
        .merge(auth::routes())
        .layer(middleware::from_fn(csrf::protect))
        .layer(sec_headers)
}
