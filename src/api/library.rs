use super::client::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryItem {
    pub asin: String,
    pub title: String,
    #[serde(default, deserialize_with = "null_vec")]
    pub authors: Vec<Person>,
    #[serde(default, deserialize_with = "null_vec")]
    pub narrators: Vec<Person>,
    #[serde(default)]
    pub runtime_length_min: Option<u64>,
    #[serde(default, deserialize_with = "null_map")]
    pub product_images: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Person { pub name: String }

#[derive(Deserialize)]
struct LibraryResp { items: Vec<LibraryItem> }

/// `num_results` is capped at 1000 server-side; a short page is the last one.
const PAGE_SIZE: usize = 1000;
/// Bound on the paging loop so a server that keeps returning full pages can't spin
/// forever. 50k titles is far beyond any real library.
const MAX_PAGES: u32 = 50;

pub async fn fetch_all(c: &Client) -> anyhow::Result<Vec<LibraryItem>> {
    let mut out = Vec::new();
    for page in 1..=MAX_PAGES {
        let r: LibraryResp = c.get(&format!(
            "1.0/library?num_results={PAGE_SIZE}&page={page}&response_groups=product_desc,product_attrs,media,contributors"
        )).await?;
        let n = r.items.len();
        out.extend(r.items);
        if n < PAGE_SIZE { return Ok(out); }
    }
    tracing::warn!("library paging hit the {MAX_PAGES}-page cap; some titles may be missing");
    Ok(out)
}

fn null_vec<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Person>, D::Error> {
    Ok(Option::deserialize(d)?.unwrap_or_default())
}
fn null_map<'de, D: serde::Deserializer<'de>>(d: D) -> Result<std::collections::HashMap<String, String>, D::Error> {
    Ok(Option::deserialize(d)?.unwrap_or_default())
}
