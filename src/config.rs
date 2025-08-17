use serde::Deserialize;
use std::{fs, time::Duration};

#[derive(Debug, Deserialize, Clone)]
pub struct ServerCfg {
    pub bind: String,
    pub cookie_name: String,
    pub session_ttl_secs: u64,
    pub pepper_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbCfg {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppCfg {
    pub server: ServerCfg,
    pub db: DbCfg,
}

impl AppCfg {
    pub fn load() -> anyhow::Result<Self> {
        let mut c = config::Config::builder()
            .add_source(config::File::with_name("config/dev").required(false))
            .add_source(config::Environment::with_prefix("OVPNADM").separator("__"))
            .build()?;
        Ok(c.try_deserialize()?)
    }
    pub fn session_ttl(&self) -> Duration {
        Duration::from_secs(self.server.session_ttl_secs)
    }
    pub fn load_pepper(&self) -> anyhow::Result<Vec<u8>> {
        let v = fs::read(&self.server.pepper_file)?;
        if v.len() < 16 { anyhow::bail!("pepper too short"); }
        Ok(v)
    }
}
