//! Cover art cache: `~/.cache/skald/covers/<asin>.jpg`.
use std::path::PathBuf;
pub fn path(asin: &str) -> PathBuf { crate::player::cache::dir().join("covers").join(format!("{asin}.jpg")) }
pub async fn ensure(asin: &str, url: &str) -> anyhow::Result<PathBuf> {
    let p = path(asin);
    if p.exists() { return Ok(p); }
    std::fs::create_dir_all(p.parent().unwrap())?;
    let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
    tokio::fs::write(&p, &bytes).await?;
    Ok(p)
}
