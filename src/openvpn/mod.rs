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
use openssl::{x509::X509Crl, asn1::Asn1Time};
use std::collections::HashMap;
use std::time::UNIX_EPOCH;
use openssl::asn1::Asn1TimeRef;

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

#[derive(serde::Serialize)]
pub struct IssuedWithStatus {
    pub serial: String,
    pub cn: String,
    pub profile: String,
    pub not_after: String,
    pub revoked: bool,
    pub revoked_at: Option<String>,
}


#[derive(Serialize)]
pub struct CcdMeta {
    pub cn: String,
    pub size: u64,
    pub modified: i64,
}

pub async fn list_ccd(st: &AppState) -> anyhow::Result<Vec<CcdMeta>> {
    let mut out = Vec::new();
    let dir = &st.cfg.ovpn.ccd_dir;

    let mut rd = tokio::fs::read_dir(dir).await?;
    while let Some(ent) = rd.next_entry().await? {
        let ty = ent.file_type().await?;
        if !ty.is_file() { continue; }

        let name = ent.file_name();
        let cn = name.to_string_lossy().to_string();
        if cn.starts_with('.') { continue; }

        let md = ent.metadata().await?;
        let size = md.len();
        let modified = md.modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        out.push(CcdMeta { cn, size, modified });
    }

    out.sort_by(|a, b| a.cn.cmp(&b.cn));
    Ok(out)
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
    if !cn_ok(&re, cn) { return Err(anyhow!("invalid CN")); }

    let path = Path::new(&st.cfg.ovpn.ccd_dir).join(cn);
    let bytes = fs::read(&path).await.unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub async fn write_ccd(st: &AppState, cn: &str, content: &str) -> Result<()> {
    let re = Regex::new(&st.cfg.ovpn.cn_pattern).unwrap();
    if !cn_ok(&re, cn) { return Err(anyhow!("invalid CN")); }

    let dir = Path::new(&st.cfg.ovpn.ccd_dir);
    fs::create_dir_all(dir).await.ok();

    let path = dir.join(cn);

    let normalized = content.replace("\r\n", "\n");

    fs::write(&path, normalized.as_bytes()).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).await.ok();
    }
    Ok(())
}



fn dec_to_hex_upper(s: &str) -> Result<String> {
    let n: u128 = s.parse()?;
    Ok(format!("{:X}", n))
}
fn asn1_to_string(t: &Asn1TimeRef) -> String {
    t.to_string()
}

async fn crl_revoked_map_dec(st: &AppState) -> Result<HashMap<String, String>> {
    let pem = vpncertd::get_crl(&st.cfg.ovpn.socket_path).await?;
    let crl = X509Crl::from_pem(pem.as_bytes())?;

    let mut m = HashMap::new();
    if let Some(all) = crl.get_revoked() {
        for r in all {
            let dec = r.serial_number().to_bn()?.to_dec_str()?.to_string();
            let when = r.revocation_date().to_string();
            m.insert(dec, when);
        }
    }
    Ok(m)
}


async fn revoked_hex_map_via_crl(st: &crate::AppState) -> Result<HashMap<String, String>> {
    let pem = crate::vpncertd::get_crl(&st.cfg.ovpn.socket_path).await?;
    let crl = X509Crl::from_pem(pem.as_bytes())
        .map_err(|e| anyhow!("parse CRL PEM: {e}"))?;
    let mut map = HashMap::new();
    if let Some(list) = crl.get_revoked() {
        for r in list {
            let hex = r.serial_number().to_bn()?.to_hex_str()?.to_string().to_uppercase();
            map.insert(hex, asn1_to_string(r.revocation_date()));
        }
    }
    Ok(map)
}



pub async fn list_issued_with_status(st: &AppState, limit: Option<usize>) -> Result<Vec<IssuedWithStatus>> {
    let issued = vpncertd::list_issued(&st.cfg.ovpn.socket_path, limit).await?;
    let rev = crl_revoked_map_dec(st).await.unwrap_or_default();

    let out = issued
        .into_iter()
        .map(|it| {
            let revoked_at = rev.get(&it.serial).cloned();
            IssuedWithStatus {
                serial: it.serial,
                cn: it.cn,
                profile: it.profile,
                not_after: it.not_after,
                revoked: revoked_at.is_some(),
                revoked_at,
            }
        })
        .collect();
    Ok(out)
}


