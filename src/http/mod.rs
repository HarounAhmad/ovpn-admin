use axum::{middleware, routing::{get, post}, Json, Router};
use axum::extract::State;
use tower_http::set_header::SetResponseHeaderLayer;
use axum::http::{HeaderValue, StatusCode};
use axum::response::Redirect;
use serde_json::{json, Value};
use crate::{openvpn, AppState};
pub mod auth; pub mod guards; pub mod csrf; pub mod admin;

pub async fn health(State(st): State<AppState>) -> Json<Value> {
    let api_ok = true;

    let daemon_ok = crate::vpncertd::health(&st.cfg.ovpn.socket_path)
        .await
        .is_ok();
    let agent_ok = false;

    Json(json!({
        "api":    { "ok": api_ok },
        "daemon": { "ok": daemon_ok },
        "agent":  { "ok": agent_ok }
    }))
}

async fn admin_ping(sess: guards::AuthSession) -> Result<&'static str, axum::http::StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"])?; Ok("pong")
}

pub fn router() -> Router<AppState> {
    let csp = "default-src 'self'; \
        style-src 'self' 'unsafe-inline'; \
        script-src 'self'; \
        connect-src 'self'; \
        img-src 'self' data:; \
        frame-ancestors 'none'; object-src 'none'; \
        base-uri 'none'; form-action 'self'";
    let sec_headers = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(csp)
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains")
        ))
        .into_inner();

    let api = Router::new()
        .route("/health", get(health))
        .route("/protected/admin-ping", get(admin_ping))
        .route("/auth/csrf", get(csrf::issue_token))
        .merge(auth::routes())
        .merge(admin::routes())
        .layer(middleware::from_fn(csrf::protect));

    Router::new()
        .route("/", get(|| async { Redirect::permanent("/ui/") }))
        .nest("/api", api)
        .merge(crate::web::spa_router())
        .layer(sec_headers)
}
