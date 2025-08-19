use axum::{
    body::{boxed, Full},
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "webui-dist"]
struct Assets;

fn resp_bytes(bytes: Vec<u8>, path: &str) -> Response {
    let mime = from_path(path).first_or_octet_stream();
    println!("Serving {} with MIME type {}", path, mime.as_ref());
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
        .body(boxed(Full::from(bytes)))
        .unwrap();
    if path != "index.html" {
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    resp
}

async fn spa_index() -> Response {
    match Assets::get("index.html") {
        Some(f) => resp_bytes(f.data.into_owned(), "index.html"),
        None => (StatusCode::NOT_FOUND, "missing index.html").into_response(),
    }
}

async fn spa_asset(Path(path): Path<String>) -> Response {
    let p = if path.is_empty() { "index.html".into() } else { path };
    if let Some(f) = Assets::get(&p) {
        return resp_bytes(f.data.into_owned(), &p);
    }
    match Assets::get("index.html") {
        Some(f) => resp_bytes(f.data.into_owned(), "index.html"),
        None => (StatusCode::NOT_FOUND, "missing index.html").into_response(),
    }
}

pub fn spa_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/ui", get(spa_index))
        .route("/ui/", get(spa_index))
        .route("/ui/*path", get(spa_asset))
}
