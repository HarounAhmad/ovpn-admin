use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode, HeaderValue},
    response::Response,
    routing::{get, post, put},
    body::{Body, boxed},
    Json, Router,
};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::{http::guards, openvpn, AppState};
use crate::http::guards::AuthSession;

#[derive(Deserialize)]
struct NewClient {
    cn: String,
    passphrase: Option<String>,
    include_key: Option<bool>,
}
#[derive(Deserialize, Default)]
struct BundleReq {
    include_key: Option<bool>,
}

#[derive(Serialize)]
struct ClientCreated {
    cn: String,
    passphrase: String,
    serial: Option<String>,
    not_after: Option<String>,
}

#[derive(Serialize)]
struct ErrorMsg { error: String }

async fn create_client(
    State(st): State<AppState>,
    sess: AuthSession,
    Json(req): Json<NewClient>,
) -> Result<Response, StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"]).map_err(|_| StatusCode::FORBIDDEN)?;

    match openvpn::create_client(&st, &req.cn, req.passphrase.as_deref()).await {
        Ok(res) => {
            let body = Json(ClientCreated {
                cn: res.cn,
                passphrase: res.passphrase,
                serial: res.serial,
                not_after: res.not_after,
            });
            let mut resp = body.into_response();
            *resp.status_mut() = StatusCode::CREATED;
            Ok(resp)
        }
        Err(e) => {
            let msg = e.to_string();
            let resp = if msg.contains("cn_exists_active") {
                (StatusCode::CONFLICT, Json(ErrorMsg { error: "cn_exists_active".into() })).into_response()
            } else if msg.contains("invalid_cn") {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorMsg { error: "invalid_cn".into() })).into_response()
            } else {
                tracing::error!("create_client: {}", msg);
                (StatusCode::BAD_GATEWAY, Json(ErrorMsg { error: "daemon_error".into() })).into_response()
            };
            Ok(resp)
        }
    }
}
async fn revoke_client(
    State(st): State<AppState>,
    sess: AuthSession,
    Path(cn): Path<String>,
) -> Result<StatusCode, StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"]).map_err(|_| StatusCode::FORBIDDEN)?;
    openvpn::revoke_client(&st, &cn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn bundle(
    State(st): State<AppState>,
    sess: AuthSession,
    Path(cn): Path<String>,
    Json(req): Json<BundleReq>,
) -> Result<Response, StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"]).map_err(|_| StatusCode::FORBIDDEN)?;
    let include_key = req.include_key.unwrap_or(false);
    let b = openvpn::build_bundle(&st, &cn, include_key)
        .await
        .map_err(|e| {
            tracing::error!("bundle({}): {}", cn, e);
            StatusCode::BAD_GATEWAY
        })?;

    let (headers, body) = openvpn::stream_file(&b.path)
        .await
        .map_err(|e| {
            tracing::error!("bundle stream({}): {}", cn, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::boxed(body))
        .unwrap();

    resp.headers_mut().extend(headers);
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", b.filename)).unwrap(),
    );

    Ok(resp)
}
#[derive(Serialize)]
struct CcdDto {
    cn: String,
    content: String,
}

async fn get_ccd(
    State(st): State<AppState>,
    sess: AuthSession,
    Path(cn): Path<String>,
) -> Result<Json<CcdDto>, StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"]).map_err(|_| StatusCode::FORBIDDEN)?;
    let content = openvpn::read_ccd(&st, &cn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CcdDto { cn, content }))
}

#[derive(Deserialize)]
struct CcdBody {
    content: String,
}

async fn put_ccd(
    State(st): State<AppState>,
    sess: AuthSession,
    Path(cn): Path<String>,
    Json(body): Json<CcdBody>,
) -> Result<StatusCode, StatusCode> {
    guards::ensure_role(&sess, &["ADMIN"]).map_err(|_| StatusCode::FORBIDDEN)?;
    openvpn::write_ccd(&st, &cn, &body.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/clients", post(create_client))
        .route("/admin/clients/:cn/revoke", post(revoke_client))
        .route("/admin/clients/:cn/bundle", post(bundle))
        .route("/admin/ccd/:cn", get(get_ccd).put(put_ccd))
}
