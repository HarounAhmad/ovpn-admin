use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;

use crate::AppState;
use crate::db;
use crate::security::password::verify_password;

#[derive(Deserialize)]
pub struct LoginForm { pub username: String, pub password: String }

#[derive(Serialize)]
pub struct Me { pub username: String, pub roles: Vec<String> }

pub fn routes() -> Router<AppState> {                  // <-- typed to AppState
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
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

async fn login(
    State(st): State<AppState>,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(user) = db::find_user_by_username(&st.db, &form.username)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    if user.disabled { return Err(StatusCode::UNAUTHORIZED); }
    if !verify_password(&form.password, &user.pw_hash, &st.pepper) { return Err(StatusCode::UNAUTHORIZED); }

    let sid = db::create_session(&st.db, &user.id, st.cfg.session_ttl().as_secs() as i64)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut c = Cookie::new(st.cfg.server.cookie_name.clone(), sid);
    c.set_path("/");
    c.set_http_only(true);
    c.set_secure(true);
    c.set_same_site(SameSite::Strict);
    c.set_max_age(CookieDuration::seconds(st.cfg.session_ttl().as_secs() as i64));

    let v = HeaderValue::from_str(&c.to_string()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::NO_CONTENT, [(axum::http::header::SET_COOKIE, v)]))
}

async fn logout(State(st): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(sid) = get_cookie(&headers, &st.cfg.server.cookie_name) {
        let _ = db::delete_session(&st.db, &sid).await;
    }
    let mut c = cookie::Cookie::new(st.cfg.server.cookie_name.clone(), "");
    c.set_path("/");
    c.set_http_only(true);
    c.set_secure(true);
    c.set_same_site(cookie::SameSite::Strict);
    c.set_max_age(cookie::time::Duration::seconds(0));
    let v = axum::http::HeaderValue::from_str(&c.to_string()).unwrap();
    (axum::http::StatusCode::NO_CONTENT, [(axum::http::header::SET_COOKIE, v)])
}
async fn me(State(st): State<AppState>, headers: HeaderMap) -> Result<Json<Me>, StatusCode> {
    let Some(sid) = get_cookie(&headers, &st.cfg.server.cookie_name) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(sess) = db::load_session(&st.db, &sid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if now >= sess.expires_at { return Err(StatusCode::UNAUTHORIZED); }

    let row = sqlx::query("SELECT username FROM users WHERE id=?")
        .bind(&sess.user_id).fetch_one(&st.db)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let uname: String = row.try_get(0).unwrap();

    let roles = db::roles_for_user(&st.db, &sess.user_id)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Me { username: uname, roles }))
}
