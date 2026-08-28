//! `POST /1.0/content/{asin}/licenserequest` + AAXC voucher decryption.
use super::client::Client;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize)]
pub struct ContentLicense {
    pub acr: String,
    pub asin: String,
    pub status_code: String,
    pub license_response: Option<String>,
    pub content_metadata: ContentMetadata,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ContentMetadata {
    pub content_url: Option<ContentUrl>,
    pub last_position_heard: Option<super::positions::LastPosition>,
    pub content_reference: Option<ContentReference>,
    pub chapter_info: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ContentUrl { pub offline_url: String }
#[derive(Debug, Clone, Deserialize)]
pub struct ContentReference { pub content_format: Option<String> }

#[derive(Deserialize)]
struct Resp { content_license: ContentLicense }

/// AES key/iv for the AAXC file, hex-encoded (what ffmpeg's `-audible_key/-audible_iv` want).
#[derive(Debug, Clone)]
pub struct Voucher { pub key: String, pub iv: String }

pub async fn request(c: &Client, asin: &str) -> anyhow::Result<ContentLicense> {
    let r: Resp = c.post(&format!("1.0/content/{asin}/licenserequest"), &json!({
        "consumption_type": "Download",
        "quality": "High",
        "supported_drm_types": ["Mpeg", "Adrm"],
        "response_groups": "last_position_heard,content_reference,chapter_info",
    })).await?;
    Ok(r.content_license)
}

/// Voucher is AES-128-CBC (no padding) with key||iv = SHA256(device_type + device_serial + customer_id + asin).
pub fn decrypt_voucher(c: &Client, lic: &ContentLicense) -> anyhow::Result<Voucher> {
    let di = &c.auth.device_info;
    let ci = &c.auth.customer_info;
    let s = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
    let seed = format!("{}{}{}{}", s(di, "device_type"), s(di, "device_serial_number"), s(ci, "user_id"), lic.asin);
    let h = Sha256::digest(seed.as_bytes());
    let (key, iv) = h.split_at(16);
    let mut buf = B64.decode(lic.license_response.as_deref().unwrap_or(""))?;
    type Dec = cbc::Decryptor<aes::Aes128>;
    Dec::new(key.into(), iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(&mut buf)
        .map_err(|e| anyhow::anyhow!("voucher decrypt: {e}"))?;
    // trailing bytes may be junk after the JSON object
    let end = buf.iter().rposition(|&b| b == b'}').map(|i| i + 1).unwrap_or(buf.len());
    #[derive(Deserialize)] struct V { key: String, iv: String }
    let v: V = serde_json::from_slice(&buf[..end])?;
    Ok(Voucher { key: v.key, iv: v.iv })
}
