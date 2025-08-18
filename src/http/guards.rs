use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};
use serde::Serialize;
use sqlx::Row;
use time::OffsetDateTime;

use crate::{db, AppState};

#[derive(Debug, Clone, Serialize)]
pub struct AuthSession {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
}

fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let h = headers.get(axum::http::header::COOKIE)?;
    let s = h.to_str().ok()?;
    for part in s.split(';') {
        let t = part.trim();
        if let Some(eq) = t.find('=') {
            if &t[..eq] == name { return Some(t[eq+1..].to_string()); }
        }
    }
    None
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthSession {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let Some(sid) = get_cookie(&parts.headers, &state.cfg.server.cookie_name) else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let Some(sess) = db::load_session(&state.db, &sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let now = OffsetDateTime::now_utc().unix_timestamp();
        if now >= sess.expires_at {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let row = sqlx::query("SELECT username FROM users WHERE id=?")
            .bind(&sess.user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let username: String = row.try_get(0).unwrap();

        let roles = db::roles_for_user(&state.db, &sess.user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(AuthSession { user_id: sess.user_id, username, roles })
    }
}

pub fn ensure_role(sess: &AuthSession, required: &[&str]) -> Result<(), StatusCode> {
    for r in required {
        if sess.roles.iter().any(|have| have == r) {
            return Ok(());
        }
    }
    Err(StatusCode::FORBIDDEN)
}
