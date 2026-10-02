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

/// Where sign-ins were kept before the keyring (and still are without one).
pub fn path() -> PathBuf {
    directories::ProjectDirs::from("dev", "turbinebmw", "skald")
        .expect("home dir")
        .config_dir()
        .join("auth.json")
}

/// The keyring item's attributes. Under omarchy-mobile's sandbox the keyring is
/// the Secret portal's: an encrypted file in the app's own home that no other
/// app can open; on a desktop, the session's Secret Service (gnome-keyring,
/// KWallet).
fn attributes() -> std::collections::HashMap<&'static str, &'static str> {
    std::collections::HashMap::from([("application", "dev.turbinebmw.Skald"), ("kind", "audible-auth")])
}

async fn keyring() -> Option<oo7::Keyring> {
    match oo7::Keyring::new().await {
        Ok(k) => Some(k),
        Err(e) => {
            tracing::warn!("no keyring ({e}); the sign-in stays in {} (0600)", path().display());
            None
        }
    }
}

impl AuthState {
    /// The sign-in: from the keyring, else from an auth.json written before
    /// it, which is then moved into the keyring and deleted.
    pub async fn load() -> anyhow::Result<Option<Self>> {
        if let Some(k) = keyring().await {
            let _ = k.unlock().await;
            if let Some(item) = k.search_items(&attributes()).await?.into_iter().next() {
                let secret = item.secret().await?;
                return Ok(Some(serde_json::from_slice(&secret)?));
            }
        }
        let p = path();
        if !p.exists() {
            return Ok(None);
        }
        tighten(&p);
        let state: Self = serde_json::from_slice(&std::fs::read(&p)?)?;
        if state.save().await.is_ok() && keyring().await.is_some() {
            let _ = std::fs::remove_file(&p);
            tracing::info!("moved the sign-in from {} into the keyring", p.display());
        }
        Ok(Some(state))
    }

    /// Keep the sign-in: the device key, the ADP token and the cookies are, together,
    /// full API access to the Amazon account. In the keyring when there is one;
    /// otherwise owner-only on disk.
    pub async fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_vec(self)?;
        if let Some(k) = keyring().await {
            let _ = k.unlock().await;
            k.create_item("Skald: Audible sign-in", &attributes(), json, true).await?;
            return Ok(());
        }
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

    /// Sign out: the keyring item and any file go.
    pub async fn forget() {
        if let Some(k) = keyring().await {
            let _ = k.delete(&attributes()).await;
        }
        let _ = std::fs::remove_file(path());
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
