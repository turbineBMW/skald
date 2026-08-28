mod accent;
mod api;
mod auth;
mod player;
mod rt;
mod ui;

use adw::prelude::*;

const APP_ID: &str = "dev.turbinebmw.Skald";

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("login") => return tokio::runtime::Runtime::new()?.block_on(cli_login(args.get(2).map(String::as_str).unwrap_or("us"))),
        Some("chapters") => return tokio::runtime::Runtime::new()?.block_on(cli_chapters(args.get(2).map(String::as_str).unwrap_or(""))),
        Some("download") => return tokio::runtime::Runtime::new()?.block_on(cli_download(args.get(2).map(String::as_str).unwrap_or(""))),
        Some("play") => return tokio::runtime::Runtime::new()?.block_on(cli_play(args.get(2).map(String::as_str).unwrap_or(""), args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8))),
        Some("probe") => return tokio::runtime::Runtime::new()?.block_on(cli_probe(args.get(2), args.get(3))),
        _ => {}
    }
    gst::init()?;
    let app = adw::Application::builder().application_id(APP_ID).flags(gtk4::gio::ApplicationFlags::NON_UNIQUE).build();
    app.connect_activate(ui::build);
    app.run_with_args::<&str>(&[]);
    Ok(())
}

/// `skald login [locale]` — headless: prints the sign-in URL, reads the redirect URL from stdin.
async fn cli_login(locale: &str) -> anyhow::Result<()> {
    let pending = auth::login::PendingLogin::start(locale);
    println!("Open this URL in a browser, sign in, then paste the final URL (the one that fails to load,\nstarting with https://www.amazon.*/ap/maplanding) below:\n\n{}\n", pending.url);
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let code = auth::login::code_from_redirect(line.trim()).ok_or_else(|| anyhow::anyhow!("no authorization code in that URL"))?;
    let state = pending.register(&code).await?;
    state.save()?;
    println!("registered; saved {}", auth::store::path().display());
    Ok(())
}

/// `skald probe [ASIN [POSITION_MS]]`
async fn cli_probe(asin: Option<&String>, ms: Option<&String>) -> anyhow::Result<()> {
    let state = auth::store::AuthState::load()?.ok_or_else(|| anyhow::anyhow!("not logged in; run `skald login`"))?;
    let c = api::client::Client::new(state)?;
    let Some(asin) = asin else {
        let lib = api::library::fetch_all(&c).await?;
        let asins: Vec<&str> = lib.iter().map(|i| i.asin.as_str()).collect();
        let pos = api::positions::read(&c, &asins).await?;
        for it in &lib {
            let p = pos.iter().find(|p| p.asin.as_deref() == Some(&it.asin)).map(|p| p.position_ms).unwrap_or(0);
            println!("{}  {:>9}ms  {}", it.asin, p, it.title);
        }
        return Ok(());
    };
    let lic = api::license::request(&c, asin).await?;
    println!("status: {}\nacr: {}\nlast_position_heard: {:?}\nformat: {:?}",
        lic.status_code, lic.acr, lic.content_metadata.last_position_heard,
        lic.content_metadata.content_reference.as_ref().and_then(|r| r.content_format.clone()));
    if let Ok(v) = api::license::decrypt_voucher(&c, &lic) { println!("voucher key/iv ok ({}…)", &v.key[..4]); }
    if let Some(ms) = ms {
        let ms: u64 = ms.parse()?;
        api::positions::write(&c, asin, &lic.acr, ms).await?;
        println!("PUT ok; readback: {:?}", api::positions::read(&c, &[asin]).await?);
    }
    Ok(())
}

#[allow(dead_code)]
async fn cli_chapters(asin: &str) -> anyhow::Result<()> {
    let state = auth::store::AuthState::load()?.unwrap();
    let c = api::client::Client::new(state)?;
    let lic = api::license::request(&c, asin).await?;
    println!("{}", serde_json::to_string_pretty(&lic.content_metadata.chapter_info)?);
    Ok(())
}

/// `skald download ASIN` — fetch + decrypt into the cache.
async fn cli_download(asin: &str) -> anyhow::Result<()> {
    let state = auth::store::AuthState::load()?.ok_or_else(|| anyhow::anyhow!("not logged in"))?;
    let c = api::client::Client::new(state)?;
    let mut last = 0;
    let (path, lic) = player::cache::ensure(&c, asin, |done, total| {
        let pct = total.map(|t| done * 100 / t).unwrap_or(0);
        if pct / 10 != last { last = pct / 10; eprintln!("{pct}%"); }
    }).await?;
    println!("{}  (server position {:?})", path.display(), lic.content_metadata.last_position_heard.map(|p| p.position_ms));
    Ok(())
}

/// `skald play ASIN [SECS]` — headless smoke test: resume from server position, play SECS, push position.
async fn cli_play(asin: &str, secs: u64) -> anyhow::Result<()> {
    gst::init()?;
    let state = auth::store::AuthState::load()?.ok_or_else(|| anyhow::anyhow!("not logged in"))?;
    let c = api::client::Client::new(state)?;
    let (path, lic) = player::cache::ensure(&c, asin, |_, _| {}).await?;
    let start = lic.content_metadata.last_position_heard.as_ref().map(|p| p.position_ms).unwrap_or(0);
    let engine = player::engine::Engine::new();
    engine.load(&path);
    engine.play();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    engine.seek_ms(start);
    println!("playing {} from {start}ms for {secs}s (dur {:?})", path.display(), engine.duration_ms());
    tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
    let pos = engine.position_ms().unwrap_or(start);
    engine.pause();
    api::positions::write(&c, asin, &lic.acr, pos).await?;
    println!("pushed {pos}ms");
    Ok(())
}
