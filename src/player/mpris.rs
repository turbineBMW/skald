//! MPRIS2 via `mpris-server` so media keys, the shell widget and `playerctl` work.
use crate::ui::AppRef;
use adw::prelude::*;
use gtk4 as gtk;
use gtk::glib;
use mpris_server::{Metadata, PlaybackStatus, Player, Time};

pub fn attach(app: &AppRef) {
    glib::spawn_future_local(glib::clone!(#[strong] app, async move {
        let player = match Player::builder("skald").identity("Skald").desktop_entry("dev.turbinebmw.Skald")
            .can_play(true).can_pause(true).can_seek(true).can_control(true).can_go_next(true).can_go_previous(true)
            .can_quit(true).can_raise(true).build().await
        {
            Ok(p) => p,
            Err(e) => { tracing::warn!("mpris unavailable: {e}"); return; }
        };
        let act = |name: &'static str| glib::clone!(#[strong] app, move |_: &Player| { app.window.application().unwrap().activate_action(name, None); });
        player.connect_play_pause(act("play-pause"));
        player.connect_play(glib::clone!(#[strong] app, move |_| if !app.engine.is_playing() { crate::ui::player::toggle_play(&app); }));
        player.connect_pause(glib::clone!(#[strong] app, move |_| app.engine.pause()));
        player.connect_stop(glib::clone!(#[strong] app, move |_| app.engine.pause()));
        player.connect_next(act("next-chapter"));
        player.connect_previous(act("prev-chapter"));
        player.connect_seek(glib::clone!(#[strong] app, move |_, off| { app.engine.skip_ms(off.as_millis()); }));
        player.connect_set_position(glib::clone!(#[strong] app, move |_, _, pos| app.engine.seek_ms(pos.as_millis().max(0) as u64)));
        player.connect_raise(glib::clone!(#[strong] app, move |_| app.window.present()));
        player.connect_quit(glib::clone!(#[strong] app, move |_| app.window.close()));
        let task = player.run();
        let player = std::rc::Rc::new(player);
        *app.mpris.borrow_mut() = Some(player.clone());
        task.await;
    }));
}

pub fn set_track(app: &AppRef, item: &crate::api::library::LibraryItem, duration_ms: Option<u64>) {
    let Some(p) = app.mpris.borrow().clone() else { return };
    let mut md = Metadata::builder().title(item.title.clone())
        .artist(item.authors.iter().map(|a| a.name.clone()).collect::<Vec<_>>())
        .trackid(mpris_server::TrackId::try_from(format!("/dev/turbinebmw/Skald/{}", item.asin.replace(|c: char| !c.is_ascii_alphanumeric(), "_"))).unwrap());
    if let Some(d) = duration_ms { md = md.length(Time::from_millis(d as i64)); }
    let art = crate::api::covers::path(&item.asin);
    if art.exists() { md = md.art_url(format!("file://{}", art.display())); }
    glib::spawn_future_local(async move { let _ = p.set_metadata(md.build()).await; });
}
pub fn set_playing(app: &AppRef, playing: bool) {
    let Some(p) = app.mpris.borrow().clone() else { return };
    glib::spawn_future_local(async move { let _ = p.set_playback_status(if playing { PlaybackStatus::Playing } else { PlaybackStatus::Paused }).await; });
}
pub fn set_position(app: &AppRef, ms: u64) {
    if let Some(p) = app.mpris.borrow().as_ref() { p.set_position(Time::from_millis(ms as i64)); }
}
pub fn seeked(app: &AppRef, ms: u64) {
    let Some(p) = app.mpris.borrow().clone() else { return };
    glib::spawn_future_local(async move { let _ = p.seeked(Time::from_millis(ms as i64)).await; });
}
pub fn set_rate(app: &AppRef, rate: f64) {
    let Some(p) = app.mpris.borrow().clone() else { return };
    glib::spawn_future_local(async move { let _ = p.set_rate(rate).await; });
}
