use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub locale: String,
    pub adp_token: String,
    pub device_private_key: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires: f64,
    pub website_cookies: serde_json::Map<String, serde_json::Value>,
    pub customer_info: serde_json::Value,
    pub device_info: serde_json::Value,
}

pub fn path() -> PathBuf {
    directories::ProjectDirs::from("dev", "turbinebmw", "skald")
        .expect("home dir")
        .config_dir()
        .join("auth.json")
}

impl AuthState {
    pub fn load() -> anyhow::Result<Option<Self>> {
        let p = path();
        if !p.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&std::fs::read(p)?)?))
    }
    /// Written owner-only: this file holds the device private key, the ADP token and
    /// the refresh token, which together are full API access to the Amazon account.
    pub fn save(&self) -> anyhow::Result<()> {
        let p = path();
        let dir = p.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        let mut f = std::fs::OpenOptions::new()
            .write(true).create(true).truncate(true).mode(0o600).open(&p)?;
        f.write_all(&serde_json::to_vec_pretty(self)?)?;
        f.flush()?;
        // `mode` only applies when the file is created; re-tighten an older 0644 one.
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }
}
