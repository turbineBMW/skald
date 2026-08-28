use crate::auth::{signer::AdpSigner, store::AuthState};
use anyhow::Context;
use reqwest::Method;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Signed client for `https://api.audible.<tld>/1.0/…`.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    signer: Arc<AdpSigner>,
    base: String,
    pub auth: Arc<AuthState>,
}

pub fn tld_for(locale: &str) -> &'static str {
    match locale {
        "us" => "com", "uk" => "co.uk", "de" => "de", "fr" => "fr", "ca" => "ca",
        "au" => "com.au", "it" => "it", "in" => "in", "jp" => "co.jp", "es" => "es",
        "br" => "com.br", _ => "com",
    }
}

impl Client {
    pub fn new(auth: AuthState) -> anyhow::Result<Self> {
        let signer = AdpSigner::new(&auth.device_private_key, &auth.adp_token)?;
        Ok(Self {
            http: reqwest::Client::builder().user_agent("Audible/3.56.2 CFNetwork Darwin").build()?,
            signer: Arc::new(signer),
            base: format!("https://api.audible.{}", tld_for(&auth.locale)),
            auth: Arc::new(auth),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path_query: &str) -> anyhow::Result<T> {
        self.call(Method::GET, path_query, None).await
    }
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &serde_json::Value) -> anyhow::Result<T> {
        self.call(Method::POST, path, Some(body)).await
    }
    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: &serde_json::Value) -> anyhow::Result<T> {
        self.call(Method::PUT, path, Some(body)).await
    }

    async fn call<T: DeserializeOwned>(&self, method: Method, path_query: &str, body: Option<&serde_json::Value>) -> anyhow::Result<T> {
        let path_query = format!("/{}", path_query.trim_start_matches('/'));
        let body_str = body.map(|b| b.to_string()).unwrap_or_default();
        let (_date, sig) = self.signer.sign(method.as_str(), &path_query, &body_str);
        let mut req = self.http
            .request(method.clone(), format!("{}{}", self.base, path_query))
            .header("x-adp-token", self.signer.adp_token())
            .header("x-adp-alg", "SHA256withRSA:1.0")
            .header("x-adp-signature", sig)
            .header("Accept", "application/json");
        if body.is_some() {
            req = req.header("Content-Type", "application/json").body(body_str);
        }
        let resp = req.send().await.context("request failed")?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("{method} {path_query} -> {status}: {text}");
        }
        let text = if text.trim().is_empty() { "null".to_owned() } else { text };
        serde_json::from_str(&text).with_context(|| format!("decoding {path_query}: {}", &text[..text.len().min(300)]))
    }
}
