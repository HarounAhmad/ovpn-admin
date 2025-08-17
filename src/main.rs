mod config;
mod db;
mod http;

use crate::config::AppCfg;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<AppCfg>,
    pub pepper: Arc<Vec<u8>>,
    pub db: db::Db,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let cfg = std::sync::Arc::new(AppCfg::load()?);
    let pepper = std::sync::Arc::new(cfg.load_pepper()?);
    let db = db::connect_db(&cfg.db.url).await?;
    db::migrate_db(&db).await?;

    let state = AppState { cfg: cfg.clone(), pepper, db };
    let app = http::router::<AppState>().with_state(state);

    let addr: SocketAddr = cfg.server.bind.parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
