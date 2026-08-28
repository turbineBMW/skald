//! `~/.cache/skald/<asin>.m4b` — decrypted, remuxed once via ffmpeg (`-c copy`, lossless).
use crate::api::{client::Client, license};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub fn dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "turbinebmw", "skald").expect("home").cache_dir().to_path_buf()
}
pub fn m4b_path(asin: &str) -> PathBuf { dir().join(format!("{asin}.m4b")) }

/// The half-written files an interrupted download would leave behind. Kept next to
/// `m4b_path` so the "remove download" paths can sweep them without duplicating the
/// naming. The remux target keeps the `.m4b` suffix so ffmpeg still picks the muxer
/// from the extension.
pub fn scratch_paths(asin: &str) -> [PathBuf; 2] {
    [dir().join(format!("{asin}.aaxc")), dir().join(format!("{asin}.part.m4b"))]
}

/// Deletes its path on drop unless disarmed with `keep`. This covers an error partway
/// through *and* the whole task being aborted when the user opens a different book —
/// dropping the future runs `Drop`, so neither leaves a stray part-file behind.
struct Scratch(Option<PathBuf>);

impl Scratch {
    fn new(p: PathBuf) -> Self { Self(Some(p)) }
    fn path(&self) -> &Path { self.0.as_deref().expect("armed") }
    /// The file has been renamed away; there is nothing left to clean up.
    fn keep(mut self) { self.0.take(); }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() { let _ = std::fs::remove_file(p); }
    }
}

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

    let [aaxc, part] = scratch_paths(asin).map(Scratch::new);
    let resp = reqwest::Client::builder().user_agent("Audible/3.56.2 CFNetwork Darwin").build()?
        .get(&url).send().await?.error_for_status()?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(aaxc.path()).await?;
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

    // Remux into a part-file and promote it with a rename, which is atomic: a half-written
    // m4b can then never be mistaken for a finished download by the `out.exists()` check
    // above. `kill_on_drop` matters for the same reason — without it, aborting this task
    // leaves ffmpeg running and still writing to that path.
    let status = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error",
               "-audible_key", &voucher.key, "-audible_iv", &voucher.iv,
               "-i"]).arg(aaxc.path())
        .args(["-map", "0:a", "-c", "copy", "-movflags", "+faststart"]).arg(part.path())
        .kill_on_drop(true)
        .status().await?;
    if !status.success() { anyhow::bail!("ffmpeg failed: {status}"); }
    std::fs::rename(part.path(), &out)?;
    part.keep();
    Ok((out, lic))
}
