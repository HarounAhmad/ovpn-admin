use crate::{db, vpncertd, AppState};
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use rand::RngCore;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue};
use tokio::{fs, io::AsyncWriteExt};
use tokio_util::io::ReaderStream;
use ulid::Ulid;



fn cn_ok(re: &Regex, cn: &str) -> bool {
    re.is_match(cn)
}

#[derive(Debug, Serialize)]
pub struct ClientIssue {
    pub cn: String,
    pub passphrase: String,
    pub cert_pem: String,
    pub key_pem_encrypted: String,
    pub serial: Option<String>,
    pub not_after: Option<String>,
}

pub async fn create_client(st: &AppState, cn: &str, passphrase: Option<&str>) -> Result<ClientIssue> {
    let re = Regex::new(&st.cfg.ovpn.cn_pattern).unwrap();
    if !re.is_match(cn) {
        return Err(anyhow!("invalid_cn"));
    }

    let pass = match passphrase {
        Some(p) => p.to_string(),
        None => {
            let mut raw = [0u8; 16];
            rand::rngs::OsRng.fill_bytes(&mut raw);
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
        }
    };

    let issued = vpncertd::genkey_and_sign(&st.cfg.ovpn.socket_path, cn, "client", "rsa4096", &pass).await?;

    db::audit_record(&st.db, "system", "CLIENT_CREATE", cn, "-", "-", "{}").await.ok();

    Ok(ClientIssue {
        cn: cn.to_string(),
        passphrase: pass,
        cert_pem: issued.cert_pem,
        key_pem_encrypted: issued.key_pem_encrypted,
        serial: issued.serial,
        not_after: issued.not_after,
    })
}

pub async fn revoke_client(st: &AppState, cn: &str) -> Result<()> {
    let re = Regex::new(&st.cfg.ovpn.cn_pattern).unwrap();
    if !cn_ok(&re, cn) {
        return Err(anyhow!("invalid CN"));
    }
    vpncertd::revoke(&st.cfg.ovpn.socket_path, cn).await?;
    db::audit_record(&st.db, "system", "CLIENT_REVOKE", cn, "-", "-", "{}")
        .await
        .ok();
    Ok(())
}

pub struct BundleFile { pub path: String, pub filename: String }

pub async fn build_bundle(st: &AppState, cn: &str, include_key: bool) -> anyhow::Result<BundleFile> {
    use std::path::Path;
    use tokio::{fs, io::AsyncWriteExt};

    let bytes = vpncertd::build_bundle_bytes(
        &st.cfg.ovpn.socket_path,
        cn,
        include_key,
        &st.cfg.ovpn.bundle_remote,
        st.cfg.ovpn.bundle_port,
        &st.cfg.ovpn.bundle_proto,
    ).await?;

    let dir = Path::new(&st.cfg.ovpn.bundles_dir);
    if !dir.exists() { fs::create_dir_all(dir).await?; }
    let filename = format!("{cn}.zip");
    let path = dir.join(&filename);
    let mut f = fs::File::create(&path).await?;
    f.write_all(&bytes).await?;
    Ok(BundleFile { filename, path: path.to_string_lossy().into_owned() })
}

pub async fn stream_file(path: &str) -> Result<(HeaderMap, Body)> {
    let file = tokio::fs::File::open(path).await?;
    let stream = ReaderStream::new(file);
    let body = Body::wrap_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    Ok((headers, body))
}

pub async fn read_ccd(st: &AppState, cn: &str) -> Result<String> {
    let re = Regex::new(&st.cfg.ovpn.cn_pattern).unwrap();
    if !cn_ok(&re, cn) {
        return Err(anyhow!("invalid CN"));
    }
    let p = std::path::Path::new(&st.cfg.ovpn.ccd_dir).join(cn);
    if !p.exists() {
        return Ok(String::new());
    }
    Ok(fs::read_to_string(&p).await?)
}

pub async fn write_ccd(st: &AppState, cn: &str, content: &str) -> Result<()> {
    let re = Regex::new(&st.cfg.ovpn.cn_pattern).unwrap();
    if !cn_ok(&re, cn) {
        return Err(anyhow!("invalid CN"));
    }
    if content.len() > 64 * 1024 {
        return Err(anyhow!("ccd too large"));
    }
    let p = std::path::Path::new(&st.cfg.ovpn.ccd_dir).join(cn);
    let tmp = p.with_extension("tmp");

    fs::create_dir_all(&st.cfg.ovpn.ccd_dir).await.ok();

    let mut f = fs::File::create(&tmp).await?;
    f.write_all(content.as_bytes()).await?;
    f.flush().await?;
    fs::rename(&tmp, &p).await?;

    db::audit_record(&st.db, "system", "CCD_WRITE", cn, "-", "-", "{}")
        .await
        .ok();
    Ok(())
}

