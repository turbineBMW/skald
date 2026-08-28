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

pub async fn fetch_all(c: &Client) -> anyhow::Result<Vec<LibraryItem>> {
    let mut out = Vec::new();
    for page in 1.. {
        let r: LibraryResp = c.get(&format!(
            "1.0/library?num_results=1000&page={page}&response_groups=product_desc,product_attrs,media,contributors"
        )).await?;
        let n = r.items.len();
        out.extend(r.items);
        if n < 1000 { break; }
    }
    Ok(out)
}

fn null_vec<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Person>, D::Error> {
    Ok(Option::deserialize(d)?.unwrap_or_default())
}
fn null_map<'de, D: serde::Deserializer<'de>>(d: D) -> Result<std::collections::HashMap<String, String>, D::Error> {
    Ok(Option::deserialize(d)?.unwrap_or_default())
}
