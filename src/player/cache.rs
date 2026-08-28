//! `~/.cache/skald/<asin>.m4b` — decrypted, remuxed once via ffmpeg (`-c copy`, lossless).
use crate::api::{client::Client, license};
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub fn dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "turbinebmw", "skald").expect("home").cache_dir().to_path_buf()
}
pub fn m4b_path(asin: &str) -> PathBuf { dir().join(format!("{asin}.m4b")) }

/// Ensure a playable, decrypted file exists for `asin`. `progress(downloaded, total)`.
/// Returns (path, license) — the license carries `acr` (needed for position writes) and the
/// server-side last position.
pub async fn ensure(c: &Client, asin: &str, mut progress: impl FnMut(u64, Option<u64>)) -> anyhow::Result<(PathBuf, license::ContentLicense)> {
    let lic = license::request(c, asin).await?;
    if lic.status_code != "Granted" { anyhow::bail!("license not granted: {}", lic.status_code); }
    let out = m4b_path(asin);
    if out.exists() { return Ok((out, lic)); }

    std::fs::create_dir_all(dir())?;
    let voucher = license::decrypt_voucher(c, &lic)?;
    let url = lic.content_metadata.content_url.as_ref().map(|u| u.offline_url.clone())
        .ok_or_else(|| anyhow::anyhow!("no offline_url in license"))?;

    let aaxc = dir().join(format!("{asin}.aaxc"));
    let resp = reqwest::Client::builder().user_agent("Audible/3.56.2 CFNetwork Darwin").build()?
        .get(&url).send().await?.error_for_status()?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(&aaxc).await?;
    let mut stream = resp.bytes_stream();
    let mut done = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().await?;
    drop(file);

    let status = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error",
               "-audible_key", &voucher.key, "-audible_iv", &voucher.iv,
               "-i"]).arg(&aaxc)
        .args(["-map", "0:a", "-c", "copy", "-movflags", "+faststart"]).arg(&out)
        .status().await?;
    let _ = tokio::fs::remove_file(&aaxc).await;
    if !status.success() { let _ = std::fs::remove_file(&out); anyhow::bail!("ffmpeg failed: {status}"); }
    Ok((out, lic))
}
