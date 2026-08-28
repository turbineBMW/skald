use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub locale: String,
    pub adp_token: String,
    pub device_private_key: String,
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
        tighten(&p);
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
        tighten(&p);
        Ok(())
    }
}

/// Drop a credential file back to owner-only. `OpenOptions::mode` applies only when the
/// file is created, so installs that predate that are fixed here instead — on load as
/// well as on save, so an existing 0644 file is repaired at the next start rather than
/// lingering until the user happens to sign in again.
fn tighten(p: &std::path::Path) {
    let Ok(meta) = std::fs::metadata(p) else { return };
    if meta.permissions().mode() & 0o077 == 0 {
        return;
    }
    match std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)) {
        Ok(()) => tracing::warn!("tightened {} to 0600; it was group/world-readable", p.display()),
        Err(e) => tracing::warn!("could not tighten {}: {e}", p.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Files written before the bearer token was dropped carry `access_token`,
    /// `refresh_token` and `expires`. Serde must ignore them rather than fail, or
    /// upgrading would silently sign the user out.
    #[test]
    fn reads_a_pre_existing_file_with_the_dropped_fields() {
        let json = serde_json::json!({
            "locale": "us",
            "adp_token": "{enc:abc}",
            "device_private_key": "-----BEGIN RSA PRIVATE KEY-----",
            "access_token": "Atna|old",
            "refresh_token": "Atnr|old",
            "expires": 1_756_000_000.0,
            "website_cookies": {"session-id": "123"},
            "customer_info": {"user_id": "amzn1.account.X"},
            "device_info": {"device_serial_number": "SERIAL"}
        });
        let s: AuthState = serde_json::from_value(json).expect("old file still parses");
        assert_eq!(s.locale, "us");
        assert_eq!(s.device_info["device_serial_number"], "SERIAL");
    }
}
