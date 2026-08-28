//! ADP request signing: `x-adp-token`, `x-adp-alg`, `x-adp-signature`.
//! Signed string = "{METHOD}\n{PATH}\n{DATE}\n{BODY}\n{ADP_TOKEN}", RSA-SHA256 PKCS#1 v1.5.
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rsa::{RsaPrivateKey, pkcs1v15::SigningKey, pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey, signature::{SignatureEncoding, Signer}};
use sha2::Sha256;

pub struct AdpSigner {
    key: SigningKey<Sha256>,
    adp_token: String,
}

impl AdpSigner {
    pub fn new(pem: &str, adp_token: &str) -> anyhow::Result<Self> {
        let key = RsaPrivateKey::from_pkcs1_pem(pem).or_else(|_| RsaPrivateKey::from_pkcs8_pem(pem))?;
        Ok(Self { key: SigningKey::new(key), adp_token: adp_token.to_owned() })
    }

    /// Returns (date, signature) header values.
    pub fn sign(&self, method: &str, path_and_query: &str, body: &str) -> (String, String) {
        let date = chrono_now();
        let data = format!("{method}\n{path_and_query}\n{date}\n{body}\n{}", self.adp_token);
        let sig = self.key.sign(data.as_bytes());
        (date.clone(), format!("{}:{date}", B64.encode(sig.to_bytes())))
    }

    pub fn adp_token(&self) -> &str { &self.adp_token }
}

fn chrono_now() -> String {
    // ISO-8601 UTC, e.g. 2026-08-27T12:34:56Z
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let secs = now.as_secs() as i64;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
