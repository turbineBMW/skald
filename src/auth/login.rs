//! Amazon OAuth (PKCE) → `POST /auth/register` device registration, and token refresh.
use super::store::AuthState;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64URL};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const DEVICE_TYPE: &str = "A2CZJZGLK2JJVM";

fn amazon_domain(locale: &str) -> &'static str { crate::api::client::tld_for(locale) }

pub struct PendingLogin {
    pub url: String,
    pub locale: String,
    serial: String,
    verifier: String,
}

fn random_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    getrandom::fill(&mut b).expect("getrandom");
    hex::encode_upper(b)
}

impl PendingLogin {
    pub fn start(locale: &str) -> Self {
        let serial = random_hex(16);
        let client_id = hex::encode(format!("{serial}#{DEVICE_TYPE}"));
        let verifier = B64URL.encode(random_hex(32));
        let challenge = B64URL.encode(Sha256::digest(verifier.as_bytes()));
        let d = amazon_domain(locale);
        let oa2 = "http://specs.openid.net/auth/2.0";
        let params = [
            ("openid.oa2.response_type", "code"),
            ("openid.oa2.code_challenge_method", "S256"),
            ("openid.oa2.code_challenge", challenge.as_str()),
            ("openid.return_to", &format!("https://www.amazon.{d}/ap/maplanding")),
            ("openid.assoc_handle", &format!("amzn_audible_ios_{locale}")),
            ("openid.identity", "http://specs.openid.net/auth/2.0/identifier_select"),
            ("pageId", "amzn_audible_ios"),
            ("accountStatusPolicy", "P1"),
            ("openid.claimed_id", "http://specs.openid.net/auth/2.0/identifier_select"),
            ("openid.mode", "checkid_setup"),
            ("openid.ns.oa2", "http://www.amazon.com/ap/ext/oauth/2"),
            ("openid.oa2.client_id", &format!("device:{client_id}")),
            ("openid.ns.pape", "http://specs.openid.net/extensions/pape/1.0"),
            ("marketPlaceId", marketplace_id(locale)),
            ("openid.oa2.scope", "device_auth_access"),
            ("forceMobileLayout", "true"),
            ("openid.ns", oa2),
            ("openid.pape.max_auth_age", "0"),
        ];
        let qs = params.iter()
            .map(|(k, v)| format!("{k}={}", urlencoding(v)))
            .collect::<Vec<_>>().join("&");
        Self { url: format!("https://www.amazon.{d}/ap/signin?{qs}"), locale: locale.to_owned(), serial, verifier }
    }

    /// Complete registration given the `openid.oa2.authorization_code` from the maplanding redirect.
    pub async fn register(self, authorization_code: &str) -> anyhow::Result<AuthState> {
        let d = amazon_domain(&self.locale);
        let client_id = hex::encode(format!("{}#{DEVICE_TYPE}", self.serial));
        let body = json!({
            "requested_token_type": ["bearer", "mac_dms", "website_cookies", "store_authentication_cookie"],
            "cookies": {"website_cookies": [], "domain": format!(".amazon.{d}")},
            "registration_data": {
                "domain": "Device", "app_version": "3.56.2",
                "device_serial": self.serial, "device_type": DEVICE_TYPE,
                "device_name": "%FIRST_NAME%%FIRST_NAME_POSSESSIVE_STRING%%DUPE_STRATEGY_1ST%Audible for iPhone",
                "os_version": "15.0.0", "software_version": "35602678",
                "device_model": "iPhone", "app_name": "Audible"
            },
            "auth_data": {
                "client_id": client_id, "authorization_code": authorization_code,
                "code_verifier": self.verifier, "code_algorithm": "SHA-256", "client_domain": "DeviceLegacy"
            },
            "requested_extensions": ["device_info", "customer_info"]
        });
        let r: Value = reqwest::Client::new()
            .post(format!("https://api.amazon.{d}/auth/register"))
            .json(&body).send().await?.error_for_status()?.json().await?;
        let s = r.get("response").and_then(|r| r.get("success"))
            .ok_or_else(|| anyhow::anyhow!("register failed: {r}"))?;
        let tok = &s["tokens"];
        let cookies = tok["website_cookies"].as_array().cloned().unwrap_or_default()
            .into_iter()
            .filter_map(|c| Some((c["Name"].as_str()?.to_owned(), c["Value"].clone())))
            .collect();
        let expires_in = tok["bearer"]["expires_in"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(3600.0);
        Ok(AuthState {
            locale: self.locale,
            adp_token: tok["mac_dms"]["adp_token"].as_str().unwrap_or_default().to_owned(),
            device_private_key: tok["mac_dms"]["device_private_key"].as_str().unwrap_or_default().to_owned(),
            access_token: tok["bearer"]["access_token"].as_str().unwrap_or_default().to_owned(),
            refresh_token: tok["bearer"]["refresh_token"].as_str().unwrap_or_default().to_owned(),
            expires: now() + expires_in,
            website_cookies: cookies,
            customer_info: s["extensions"]["customer_info"].clone(),
            device_info: s["extensions"]["device_info"].clone(),
        })
    }
}

/// Pull `openid.oa2.authorization_code` out of the maplanding redirect URL.
pub fn code_from_redirect(url: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "openid.oa2.authorization_code").then(|| v.to_owned())
    })
}

fn marketplace_id(locale: &str) -> &'static str {
    match locale {
        "us" => "AF2M0KC94RCEA", "uk" => "A2I9A3Q2GNFNGQ", "de" => "AN7V1F1VY261K",
        "fr" => "A2728XDNODOQ8T", "ca" => "A2CQZ5RBY40XE", "au" => "AN7EY7DTAW63G",
        "it" => "A2N7FU2W2BU2ZC", "in" => "AJO3FBRUE6J4S", "jp" => "A1QAP3MOU4173J",
        "es" => "ALMIKO4SZCSAR", "br" => "A10J1VAYUDTYRN", _ => "AF2M0KC94RCEA",
    }
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

fn urlencoding(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}
