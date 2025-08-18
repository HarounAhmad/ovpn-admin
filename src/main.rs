mod config;
mod db;
mod http;
mod security;
mod vpncertd;
mod openvpn;

use crate::config::AppCfg;
use axum::Router;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<AppCfg>,
    pub pepper: Arc<Vec<u8>>,
    pub db: db::Db,
}

#[derive(Parser)]
#[command(name="ovpn-admin", version, about="OpenVPN admin panel")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    UserAdd { #[arg(long)] username: String, #[arg(long, value_parser=["ADMIN","OPS","READONLY"])] role: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).compact().init();

    let cfg = Arc::new(AppCfg::load()?);
    let pepper = Arc::new(cfg.load_pepper()?);
    let db = db::connect_db(&cfg.db.url).await?;
    db::migrate_db(&db).await?;

    let cli = Cli::parse();
    if let Some(Cmd::UserAdd { username, role }) = cli.cmd {
        let pw = rpassword::prompt_password("Password: ")?;
        let phc = security::password::hash_password(&pw, &pepper)?;
        let uid = db::create_user(&db, &username, &phc).await?;
        db::assign_role(&db, &uid, &role).await?;
        println!("created user '{}' with role '{}'", username, role);
        return Ok(());
    }

    let state = AppState { cfg: cfg.clone(), pepper, db };
    let app = http::router().with_state(state);

    let addr: std::net::SocketAddr = cfg.server.bind.parse()?;
    tracing::info!("listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await?;
    Ok(())


}
