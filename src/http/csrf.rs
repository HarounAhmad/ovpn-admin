use axum::{http::{Request, StatusCode, header}, middleware::Next, response::Response};
use cookie::Cookie;
use base64::Engine;
use rand::RngCore;
const CSRF_COOKIE: &str = "XSRF-TOKEN";
const CSRF_HEADER: &str = "X-CSRF-Token";

fn get_cookie(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let h = headers.get(header::COOKIE)?;
    let s = h.to_str().ok()?;
    for part in s.split(';') {
        let t = part.trim();
        if let Some(eq) = t.find('=') {
            if &t[..eq] == name { return Some(t[eq+1..].to_string()); }
        }
    }
    None
}

pub async fn issue_token() -> impl axum::response::IntoResponse {
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let mut c = Cookie::new(CSRF_COOKIE, token);
    c.set_path("/");
    c.set_secure(true);
    c.set_http_only(false); // double-submit cookie pattern
    c.set_same_site(cookie::SameSite::Strict);
    let v = axum::http::HeaderValue::from_str(&c.to_string()).unwrap();
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, v)])
}

pub async fn protect<B>(mut req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    use axum::http::Method;
    let m = req.method();
    let needs = matches!(m, &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE);
    if !needs {
        return Ok(next.run(req).await);
    }
    let hdr = req.headers().get(CSRF_HEADER).and_then(|v| v.to_str().ok());
    let cookie = get_cookie(req.headers(), CSRF_COOKIE);
    match (hdr, cookie.as_deref()) {
        (Some(h), Some(c)) if h == c => Ok(next.run(req).await),
        _ => Err(StatusCode::FORBIDDEN),
    }
}
