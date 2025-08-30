use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    time::{timeout, Duration},
};

async fn call_raw(socket: &str, req: &Value) -> Result<Value> {
    if !Path::new(socket).exists() { return Err(anyhow!("socket not found: {}", socket)); }
    let mut stream = UnixStream::connect(socket).await?;
    let mut buf = Vec::with_capacity(1024);
    serde_json::to_writer(&mut buf, req)?;
    buf.push(b'\n');
    timeout(Duration::from_secs(10), stream.write_all(&buf)).await??;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    timeout(Duration::from_secs(60), reader.read_line(&mut line)).await??;
    if line.trim().is_empty() { return Err(anyhow!("empty reply")); }
    let v: Value = serde_json::from_str(&line)?;
    if let Some(e) = v.get("err").and_then(|s| s.as_str()) { return Err(anyhow!(e.to_string())); }
    Ok(v)
}

#[derive(Deserialize)]
struct IssueWire {
    cert_pem: String,
    key_pem_encrypted: String,
    #[serde(default)] serial: Option<String>,
    #[serde(default)] not_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedMeta {
    pub serial: String,
    pub cn: String,
    pub profile: String,
    pub not_after: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

pub struct IssueReply {
    pub cert_pem: String,
    pub key_pem_encrypted: String,
    pub serial: Option<String>,
    pub not_after: Option<String>,
}

pub async fn genkey_and_sign(
    socket: &str, cn: &str, profile: &str, key_type: &str, passphrase: &str,
) -> Result<IssueReply> {
    let req = json!({
        "op": "GENKEY_AND_SIGN",
        "cn": cn,
        "profile": profile,
        "key_type": key_type,
        "passphrase": passphrase
    });
    let v = call_raw(socket, &req).await?;
    let w: IssueWire = serde_json::from_value(v)?;
    Ok(IssueReply {
        cert_pem: w.cert_pem,
        key_pem_encrypted: w.key_pem_encrypted,
        serial: w.serial,
        not_after: w.not_after,
    })
}

pub async fn build_bundle_bytes(
    socket: &str, cn: &str, include_key: bool, remote_host: &str, remote_port: u16, proto: &str,
) -> Result<Vec<u8>> {
    let req = json!({
        "op": "BUILD_BUNDLE",
        "bundle": {
            "cn": cn,
            "include_key": include_key,
            "remote_host": remote_host,
            "remote_port": remote_port,
            "proto": proto
        }
    });
    let v = call_raw(socket, &req).await?;
    let zip_b64 = v.get("zip_b64").and_then(|s| s.as_str()).ok_or_else(|| anyhow!("missing zip_b64"))?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(zip_b64)?;
    Ok(bytes)
}
pub async fn health(socket: &str) -> Result<()> {
    let _ = call_raw(socket, &json!({"op":"HEALTH"})).await?;
    Ok(())
}


fn looks_like_serial(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}


pub async fn revoke(socket: &str, id: &str) -> Result<()> {
    let serial = if looks_like_serial(id) {
        id.to_string()
    } else {
        let issued = list_issued(socket, None).await?;
        let mut candidates: Vec<&IssuedMeta> = issued.iter().filter(|it| it.cn == id).collect();
        if candidates.is_empty() {
            return Err(anyhow!("not_found: cn"));
        }
        // prefer highest numeric serial when multiple exist
        candidates.sort_by_key(|it| it.serial.parse::<u128>().unwrap_or(0));
        let last = candidates.last().unwrap();
        last.serial.clone()
    };

    let req = json!({
        "op": "REVOKE",
        "serial": serial,
        "reason": "keyCompromise",
    });
    let _ = call_raw(socket, &req).await?;
    Ok(())
}

pub async fn build_bundle(
    socket: &str,
    bundle_cn: &str,
    remote: &str,
    port: u16,
    proto: &str,
    include_key: bool,
    out_path: &str,
) -> Result<()> {
    let req = serde_json::json!({
        "op": "BUILD_BUNDLE",
        "bundle-cn": bundle_cn,
        "bundle-remote": remote,
        "bundle-port": port,
        "bundle-proto": proto,
        "bundle-include-key": include_key,
        "bundle-out": out_path
    });
    let _ = call_raw(socket, &req).await?;
    Ok(())
}

pub async fn list_issued(socket: &str, limit: Option<usize>) -> Result<Vec<IssuedMeta>> {
    let req = json!({ "op": "LIST_ISSUED" });
    let v = call_raw(socket, &req).await?;
    let mut list: Vec<IssuedMeta> = serde_json::from_value(
        v.get("issued").cloned().unwrap_or(Value::Array(vec![]))
    )?;
    if let Some(n) = limit {
        if list.len() > n { list.truncate(n); }
    }
    Ok(list)
}