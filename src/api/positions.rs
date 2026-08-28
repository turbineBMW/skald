//! Whispersync last-position read/write.
use super::client::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
pub struct LastPosition {
    #[serde(default)]
    pub asin: Option<String>,
    #[serde(default, deserialize_with = "num_or_str")]
    pub position_ms: u64,
    pub last_updated: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
struct Annot { asin: String, last_position_heard: LastPosition }
#[derive(Deserialize)]
struct Resp { asin_last_position_heard_annots: Vec<Annot> }

/// The `asins` query param is capped at 25 ASINs per request.
pub async fn read(c: &Client, asins: &[&str]) -> anyhow::Result<Vec<LastPosition>> {
    let mut out = Vec::new();
    for chunk in asins.chunks(25) {
        let r: Resp = c.get(&format!("1.0/annotations/lastpositions?asins={}", chunk.join(","))).await?;
        out.extend(r.asin_last_position_heard_annots.into_iter()
            .filter(|a| a.last_position_heard.status.as_deref() != Some("DoesNotExist"))
            .map(|a| LastPosition { asin: Some(a.asin), ..a.last_position_heard }));
    }
    Ok(out)
}

pub async fn write(c: &Client, asin: &str, acr: &str, position_ms: u64) -> anyhow::Result<()> {
    let _: serde_json::Value = c
        .put(&format!("1.0/lastpositions/{asin}"), &json!({"acr": acr, "asin": asin, "position_ms": position_ms}))
        .await?;
    Ok(())
}

fn num_or_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
        serde_json::Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    })
}
