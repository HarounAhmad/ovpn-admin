use axum::{
    extract::{State, ConnectInfo, Query},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::net::SocketAddr;
use time::OffsetDateTime;

use crate::AppState;
use crate::db;
use crate::security::password::verify_password;

#[derive(Deserialize)]
pub struct LoginForm { pub username: String, pub password: String }

#[derive(Serialize)]
pub struct Me { pub username: String, pub roles: Vec<String> }

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
        .route("/admin/audit", get(audit_list))
}

fn ua(headers: &HeaderMap) -> String {
    headers.get(axum::http::header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("-").to_string()
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let ip = peer.ip().to_string();
    let user_agent = ua(&headers);

    db::record_login_attempt(&st.db, &form.username, &ip).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (by_user_ip, by_ip) = db::login_counts(&st.db, &form.username, &ip, 600).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if by_user_ip > 10 || by_ip > 30 {
        let _ = db::audit_record(&st.db, &form.username, "LOGIN_THROTTLE", "-", &ip, &user_agent, "{}").await;
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let Some(user) = db::find_user_by_username(&st.db, &form.username)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else {
        let _ = db::audit_record(&st.db, &form.username, "LOGIN_FAIL_NOUSER", "-", &ip, &user_agent, "{}").await;
        return Err(StatusCode::UNAUTHORIZED);
    };
    if user.disabled {
        let _ = db::audit_record(&st.db, &form.username, "LOGIN_FAIL_DISABLED", "-", &ip, &user_agent, "{}").await;
        return Err(StatusCode::UNAUTHORIZED);
    }
    if !verify_password(&form.password, &user.pw_hash, &st.pepper) {
        let _ = db::audit_record(&st.db, &form.username, "LOGIN_FAIL_BADPW", "-", &ip, &user_agent, "{}").await;
        return Err(StatusCode::UNAUTHORIZED);
    }

    let sid = db::create_session(&st.db, &user.id, st.cfg.session_ttl().as_secs() as i64)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut c = Cookie::new(st.cfg.server.cookie_name.clone(), sid);
    c.set_path("/");
    c.set_http_only(true);
    c.set_secure(true);
    c.set_same_site(SameSite::Strict);
    c.set_max_age(CookieDuration::seconds(st.cfg.session_ttl().as_secs() as i64));
    let v = HeaderValue::from_str(&c.to_string()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = db::audit_record(&st.db, &user.username, "LOGIN_SUCCESS", "-", &ip, &user_agent, "{}").await;

    Ok((StatusCode::NO_CONTENT, [(axum::http::header::SET_COOKIE, v)]))
}

async fn logout(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let ip = peer.ip().to_string();
    let user_agent = ua(&headers);

    if let Some(sid) = get_cookie(&headers, &st.cfg.server.cookie_name) {
        if let Ok(Some(sess)) = db::load_session(&st.db, &sid).await {
            if let Ok(row) = sqlx::query("SELECT username FROM users WHERE id=?").bind(&sess.user_id).fetch_one(&st.db).await {
                let uname: String = row.try_get(0).unwrap();
                let _ = db::audit_record(&st.db, &uname, "LOGOUT", "-", &ip, &user_agent, "{}").await;
            }
        }
        let _ = db::delete_session(&st.db, &sid).await;
    }

    let mut c = Cookie::new(st.cfg.server.cookie_name.clone(), "");
    c.set_path("/");
    c.set_http_only(true);
    c.set_secure(true);
    c.set_same_site(SameSite::Strict);
    c.set_max_age(CookieDuration::seconds(0));
    let v = HeaderValue::from_str(&c.to_string()).unwrap();
    (StatusCode::NO_CONTENT, [(axum::http::header::SET_COOKIE, v)])
}

async fn me(
    State(st): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Me>, StatusCode> {
    let Some(sid) = get_cookie(&headers, &st.cfg.server.cookie_name) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(sess) = db::load_session(&st.db, &sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if now >= sess.expires_at { return Err(StatusCode::UNAUTHORIZED); }

    let row = sqlx::query("SELECT username FROM users WHERE id=?")
        .bind(&sess.user_id).fetch_one(&st.db)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let uname: String = row.try_get(0).unwrap();

    let roles = db::roles_for_user(&st.db, &sess.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Me { username: uname, roles }))
}

#[derive(Deserialize)]
struct Page { limit: Option<i64>, offset: Option<i64> }

#[derive(Serialize)]
struct AuditDto {
    ts: i64, actor_user: String, action: String, target: String, ip: String, ua: String, details: String
}

async fn audit_list(
    State(st): State<AppState>,
    Query(p): Query<Page>,
) -> Result<Json<Vec<AuditDto>>, StatusCode> {
    let limit = p.limit.unwrap_or(50);
    let offset = p.offset.unwrap_or(0);
    let rows = db::audit_list(&st.db, limit, offset).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|r| AuditDto {
        ts: r.ts, actor_user: r.actor_user, action: r.action, target: r.target, ip: r.ip, ua: r.ua, details: r.details
    }).collect();
    Ok(Json(out))
}
