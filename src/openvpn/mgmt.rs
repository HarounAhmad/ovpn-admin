use anyhow::{anyhow, bail, Context, Result};
use std::{sync::Arc, time::Duration};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, time::sleep};
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tokio::time::timeout;

#[derive(Clone, Debug, serde::Serialize)]
pub struct MgmtClientRow {
    pub cn: String,
    pub real_ip: String,
    pub vpn_ip: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub connected_since: String,
    pub client_id: Option<u32>,
    pub peer_id: Option<u32>,
}

#[derive(Clone)]
pub struct MgmtState {
    addr: String,
    data: Arc<RwLock<Vec<MgmtClientRow>>>,
}

impl MgmtState {
    pub fn snapshot(&self) -> Arc<RwLock<Vec<MgmtClientRow>>> {
        self.data.clone()
    }
    pub fn addr(&self) -> &str { &self.addr }
}

pub fn start(addr: String, poll_secs: u64) -> MgmtState {
    let st = MgmtState {
        addr: addr.clone(),
        data: Arc::new(RwLock::new(Vec::new())),
    };
    let out = st.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(rows) = fetch_status(&addr).await {
                *st.data.write().await = rows;
            }
            sleep(Duration::from_secs(poll_secs.max(2))).await;
        }
    });
    out
}

pub async fn health(addr: &str) -> bool {
    if addr.is_empty() {
        return false;
    }
    let Ok(Ok(mut s)) = timeout(Duration::from_secs(1), UnixStream::connect(addr)).await else {
        return false;
    };

    let _ = timeout(Duration::from_millis(200), async {
        let mut tmp = [0u8; 512];
        let _ = s.read(&mut tmp).await;
    }).await;

    if timeout(Duration::from_millis(500), s.write_all(b"load-stats\n")).await.is_err() {
        return false;
    }
    let _ = timeout(Duration::from_millis(200), s.flush()).await;

    let Ok(Ok(n)) = timeout(Duration::from_secs(1), async {
        let mut tmp = [0u8; 1024];
        s.read(&mut tmp).await.map(|n| (tmp, n))
    }).await else {
        return false;
    };

    let (buf, n) = n;
    let line = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
    line.starts_with("success:") || line.contains("connected") || line.contains("hold")
}

async fn fetch_status(addr: &str) -> Result<Vec<MgmtClientRow>> {
    let mut s = UnixStream::connect(addr)
        .await
        .with_context(|| format!("connect mgmt sock {}", addr))?;

    s.write_all(b"status 3\nquit\n" ).await?;
    s.flush().await?;

    let mut buf = Vec::new();
    let mut tmp = [0u8; 4098];

    let end_seen = |b: &[u8]| {
        std::str::from_utf8(b)
            .ok()
            .map(|t| t.lines().any(|l| l.trim_end() == "END"))
            .unwrap_or(false)
    };

    loop {
        let n = s.read(&mut tmp).await?;
        if n == 0 {
            break;
        }

        buf.extend_from_slice(&tmp[..n]);
        if end_seen(&buf) {
            break;
        }
        if buf.len() > 2 * 1024 * 1024 {
            anyhow::bail!("mgmt response too large");
        }
    }
    let text = String::from_utf8_lossy(&buf);
    Ok(parse_status_csv(&text))
}

fn parse_status_csv(s: &str) -> Vec<MgmtClientRow> {
    let mut out = Vec::new();

    let mut has_header = false;

    for line in s.lines() {
        if line.starts_with("HEADER,CLIENT_LIST") {
            has_header = true;
            continue;
        }
        if !line.starts_with("CLIENT_LIST,") {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 7 {
            continue;
        }

        let cn = parts[1].to_string();
        let real_ip = parts[2].to_string();
        let vpn_ip = parts[3].to_string();
        let bytes_in = parts.get(4).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let bytes_out = parts.get(5).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let connected_since = parts.get(6).map(|s| s.to_string()).unwrap_or_default();

        let client_id = parts
            .get(9).and_then(|v| v.parse::<u32>().ok())
            .or_else(|| parts.get(10).and_then(|v| v.parse::<u32>().ok()))
            .or_else(|| parts.get(7).and_then(|v| v.parse::<u32>().ok()));

        let peer_id = parts
            .get(10).and_then(|v| v.parse::<u32>().ok())
            .or_else(|| parts.get(11).and_then(|v| v.parse::<u32>().ok()));

        out.push(MgmtClientRow {
            cn,
            real_ip,
            vpn_ip,
            bytes_in,
            bytes_out,
            connected_since,
            client_id,
            peer_id,
        });
    }

    out
}

#[derive(Debug)]
pub enum KickTarget {
    ById(u32),
    ByCn(String),
}

pub async fn kick(m: &MgmtState, target: KickTarget) -> Result<()> {
    let cmd = match target {
        KickTarget::ById(id) => format!("kill {}\n", id),
        KickTarget::ByCn(cn) => {
            let snap_arc = m.snapshot();
            let snap = snap_arc.read().await;
            let id = snap
                .iter()
                .find(|r| r.cn == cn)
                .and_then(|r| r.client_id)
                .ok_or_else(|| anyhow!("client_not_found"))?;
            format!("kill {}\n", id)
        }
    };

    let path = m.addr.to_owned();

    let mut s = UnixStream::connect(&path)
        .await
        .with_context(|| format!("connect mgmt sock {}", path))?;
    let mut tmp = [0u8; 1024];
    let _ = timeout(Duration::from_millis(200), s.read(&mut tmp)).await;
    s.write_all(cmd.as_bytes()).await?;
    s.flush().await?;

    let n = timeout(Duration::from_millis(800), s.read(&mut tmp))
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(0);
    let reply = String::from_utf8_lossy(&tmp[..n]).to_lowercase();

    if reply.is_empty()
        || reply.contains("success")
        || reply.contains("killed")
        || reply.contains("client-kill")
    {
        Ok(())
    } else if reply.contains("error") || reply.contains("fail") {
        bail!("mgmt kick error: {}", reply.trim());
    } else {
        Ok(())
    }
}