use super::{App, AppRef, Current, Resume, debug, fmt_ms, toast};
use crate::api::{chapters, covers, library::LibraryItem, positions};
use crate::player::cache;
use adw::prelude::*;
use gtk4 as gtk;
use gtk::glib;
use std::cell::Cell;
use std::rc::Rc;

/// How close the reported position must get to the resume target before pushes are
/// armed. Generous, because the first update after a seek can land slightly short.
const LANDED_TOLERANCE_MS: u64 = 5_000;

pub struct PlayerPage {
    pub page: adw::NavigationPage,
    pub cover: gtk::Picture,
    pub title: gtk::Label,
    pub author: gtk::Label,
    pub chapter: gtk::Label,
    pub scale: gtk::Scale,
    pub elapsed: gtk::Label,
    pub remaining: gtk::Label,
    pub play_btn: gtk::Button,
    /// The speed button in the action row, and the chosen speed's index.
    pub speed: gtk::Button,
    pub speed_index: Cell<usize>,
    pub chapter_list: gtk::ListBox,
    /// The chapters, a page of their own (the action row's Chapters).
    pub chapters_page: adw::NavigationPage,
    pub status: gtk::Label,
    pub seeking: Cell<bool>,
    /// What `poll` last saw the pipeline doing.
    pub playing: Cell<bool>,
    pub speeds: Vec<f64>,
}

impl PlayerPage {
    pub fn new() -> Self {
        let cover = gtk::Picture::builder().width_request(260).height_request(260).content_fit(gtk::ContentFit::Cover).can_shrink(true)
            .halign(gtk::Align::Center).css_classes(["card"]).build();
        let title = gtk::Label::builder().wrap(true).justify(gtk::Justification::Center).css_classes(["title-2"]).build();
        let author = gtk::Label::builder().css_classes(["dim-label"]).build();
        let chapter = gtk::Label::builder().css_classes(["heading"]).margin_top(12).build();
        let scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal).hexpand(true).draw_value(false).build();
        scale.set_range(0.0, 1.0);
        let elapsed = gtk::Label::builder().label("0:00").css_classes(["numeric", "caption"]).width_chars(8).xalign(0.0).build();
        let remaining = gtk::Label::builder().label("-0:00").css_classes(["numeric", "caption"]).width_chars(8).xalign(1.0).build();
        let times = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        times.append(&elapsed); times.append(&scale); times.append(&remaining);

        let btn = |icon: &str, tip: &str| gtk::Button::builder().icon_name(icon).tooltip_text(tip).css_classes(["circular", "flat"]).valign(gtk::Align::Center).build();
        let prev = btn("media-skip-backward-symbolic", "Previous chapter");
        let back = btn("media-seek-backward-symbolic", "Back 30s");
        let play_btn = gtk::Button::builder().icon_name("media-playback-start-symbolic").css_classes(["circular", "suggested-action"])
            .width_request(64).height_request(64).build();
        let fwd = btn("media-seek-forward-symbolic", "Forward 30s");
        let next = btn("media-skip-forward-symbolic", "Next chapter");
        prev.set_action_name(Some("app.prev-chapter")); back.set_action_name(Some("app.back"));
        play_btn.set_action_name(Some("app.play-pause")); fwd.set_action_name(Some("app.forward")); next.set_action_name(Some("app.next-chapter"));
        let controls = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12).halign(gtk::Align::Center).build();
        for w in [&prev, &back, &play_btn, &fwd, &next] { controls.append(w); }

        let speeds = vec![0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5];
        let speed = gtk::Button::builder().label("1×").action_name("app.speed").build();

        let status = gtk::Label::builder().css_classes(["dim-label", "caption"]).build();

        let column = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).margin_top(18).margin_bottom(18)
            .margin_start(18).margin_end(18).valign(gtk::Align::Center).build();
        for w in [cover.upcast_ref::<gtk::Widget>(), title.upcast_ref(), author.upcast_ref(), chapter.upcast_ref(),
                  times.upcast_ref(), controls.upcast_ref(), status.upcast_ref()] { column.append(w); }
        let scroll = gtk::ScrolledWindow::builder().child(&adw::Clamp::builder().child(&column).build())
            .hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();

        // The page: the title only at the top (back is the shell's edge
        // swipe); Chapters, the speed and Remove download in the action row.
        let page = omarchy_mobile::page("Now playing", "player", &scroll);
        let actions = omarchy_mobile::Actions::new();
        actions.add("Chapters", "app.chapters");
        actions.add_button(&speed);
        actions.add("Remove download", "app.remove-download");
        omarchy_mobile::set_actions(&page, &actions);

        let chapter_list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Single).css_classes(["boxed-list"]).build();
        let chapters_scroll = gtk::ScrolledWindow::builder()
            .child(&adw::Clamp::builder().child(&chapter_list).margin_top(12).margin_bottom(18).margin_start(12).margin_end(12).build())
            .hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
        let chapters_page = omarchy_mobile::page("Chapters", "chapters", &chapters_scroll);

        Self { page, cover, title, author, chapter, scale, elapsed, remaining, play_btn, speed, speed_index: Cell::new(1), chapter_list, chapters_page, status, seeking: Cell::new(false), playing: Cell::new(false), speeds }
    }
}

pub fn attach(app: &AppRef) {
    let gtk_app = app.window.application().unwrap();
    let add = |name: &str, f: Box<dyn Fn(&AppRef)>| {
        let a = gtk::gio::SimpleAction::new(name, None);
        a.connect_activate(glib::clone!(#[strong] app, move |_, _| f(&app)));
        gtk_app.add_action(&a);
    };
    add("play-pause", Box::new(toggle_play));
    add("back", Box::new(|a| { a.engine.skip_ms(-30_000); after_seek(a); }));
    add("forward", Box::new(|a| { a.engine.skip_ms(30_000); after_seek(a); }));
    add("prev-chapter", Box::new(|a| jump_chapter(a, -1)));
    add("chapters", Box::new(|a| a.nav.push(&a.player_page.chapters_page)));
    add("speed", Box::new(choose_speed));
    add("remove-download", Box::new(|a| super::confirm(a, "Remove this download?", "The book is deleted from the phone; it downloads again when you open it.", "Remove", remove_download)));
    // Re-render the library on return so download badges / progress are current.
    app.nav.connect_visible_page_notify(glib::clone!(#[strong] app, move |nav| {
        if nav.visible_page().and_then(|p| p.tag()).as_deref() == Some("library") && !app.library.borrow().is_empty() {
            super::library::render(&app);
        }
    }));
    add("next-chapter", Box::new(|a| jump_chapter(a, 1)));
    // Keyboard shortcuts. Bubble phase so a focused entry (search box) consumes Space/arrows first.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Bubble);
    keys.connect_key_pressed(glib::clone!(#[strong] app, move |_, key, _, _| {
        if GtkWindowExt::focus(&app.window).is_some_and(|w| w.is::<gtk::Text>() || w.is::<gtk::Editable>()) { return glib::Propagation::Proceed; }
        let action = match key {
            gtk::gdk::Key::space => "play-pause",
            gtk::gdk::Key::Left => "back",
            gtk::gdk::Key::Right => "forward",
            _ => return glib::Propagation::Proceed,
        };
        app.window.application().unwrap().activate_action(action, None);
        glib::Propagation::Stop
    }));
    app.window.add_controller(keys);

    let pp = &app.player_page;
    pp.chapter_list.connect_row_activated(glib::clone!(#[strong] app, move |_, row| {
        let idx = row.index() as usize;
        let start = app.current.borrow().as_ref().and_then(|c| c.chapters.get(idx)).map(|c| c.start_offset_ms);
        if let Some(ms) = start { app.engine.seek_ms(ms); after_seek(&app); }
        // Back to the player, at that chapter.
        if app.nav.visible_page().and_then(|p| p.tag()).as_deref() == Some("chapters") { app.nav.pop(); }
    }));
    // Scale: user drag → seek on release.
    let gesture = gtk::GestureClick::new();
    gesture.connect_pressed(glib::clone!(#[strong] app, move |_, _, _, _| app.player_page.seeking.set(true)));
    gesture.connect_released(glib::clone!(#[strong] app, move |_, _, _, _| {
        app.player_page.seeking.set(false);
        app.engine.seek_ms(app.player_page.scale.value() as u64);
        after_seek(&app);
    }));
    pp.scale.add_controller(gesture);
    pp.scale.connect_value_changed(glib::clone!(#[strong] app, move |s| {
        if app.player_page.seeking.get() { update_time_labels(&app, s.value() as u64); }
    }));

    // GStreamer bus → UI.
    let bus = app.engine.message_bus();
    let guard = bus.add_watch_local(glib::clone!(#[strong] app, move |_, msg| {
        use gst_play::PlayMessage as M;
        debug(|| format!("bus: {:?}", msg.structure().map(|s| s.name().to_string())));
        match M::parse(msg) {
            Ok(M::PositionUpdated(p)) => { if let Some(t) = p.position() { on_position(&app, t.mseconds()); } }
            Ok(M::DurationChanged(d)) => { if let Some(t) = d.duration() { app.player_page.scale.set_range(0.0, t.mseconds() as f64); } }
            Ok(M::StateChanged(s)) => on_state(&app, s.state()),
            Ok(M::EndOfStream(_)) => { app.engine.pause(); }
            Ok(M::Error(e)) => toast(&app, &format!("playback error: {}", e.error())),
            _ => {}
        }
        glib::ControlFlow::Continue
    })).expect("bus watch");
    std::mem::forget(guard);

    // The pipeline's own position, length and state, four times a second, so
    // the page and the resume follow it even when a GstPlay message doesn't
    // come (they didn't, on the phone, and the book never started).
    glib::timeout_add_local(std::time::Duration::from_millis(250), glib::clone!(#[strong] app, move || {
        poll(&app);
        glib::ControlFlow::Continue
    }));
}

/// What the pipeline says now: drives the resume, the scale and the play button.
fn poll(app: &AppRef) {
    if app.current.borrow().is_none() { return; }
    let playing = app.engine.is_playing();
    if app.player_page.playing.get() != playing {
        app.player_page.playing.set(playing);
        debug(|| format!("playing: {playing}"));
        app.player_page.play_btn.set_icon_name(if playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" });
        crate::player::mpris::set_playing(app, playing);
        if !playing && app.resume.get() == Resume::Armed {
            if let Some(ms) = app.engine.position_ms() { push_position(app, ms, true); }
        }
    }
    if let Some(d) = app.engine.duration_ms() {
        let (_, max) = (app.player_page.scale.adjustment().lower(), app.player_page.scale.adjustment().upper());
        if (max - d as f64).abs() > 1.0 { app.player_page.scale.set_range(0.0, d as f64); }
    }
    let Some(ms) = app.engine.position_ms() else { return };
    // Playing: now the resume seek lands (a flushing seek while paused left
    // the PipeWire sink never prerolling again).
    if let Resume::Prerolling(start) = app.resume.get() {
        if !playing { return; }
        debug(|| format!("playing at {ms} ms; seeking to {start} ms"));
        if start > 0 {
            app.resume.set(Resume::Seeking(start));
            app.engine.seek_ms(start);
        } else {
            app.resume.set(Resume::Armed);
            app.engine.set_volume(1.0);
        }
        let rate = app.player_page.speeds[app.player_page.speed_index.get()];
        if rate != 1.0 { app.engine.set_rate(rate); }
        return;
    }
    on_position(app, ms);
}

/// Open a book: ensure it's cached (download+decrypt), then load and resume at the server position.
pub fn open(app: &AppRef, item: LibraryItem) {
    let Some(client) = app.client.borrow().clone() else { return };
    // Flush the outgoing book's position first.
    if let Some(ms) = app.engine.position_ms() { push_position(app, ms, true); }
    app.resume.set(Resume::Idle);
    app.engine.pause();

    let pp = &app.player_page;
    pp.title.set_label(&item.title);
    let by = item.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ");
    let read_by = item.narrators.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ");
    pp.author.set_label(&if read_by.is_empty() { by } else { format!("{by}  ·  read by {read_by}") });
    pp.chapter.set_label("");
    pp.status.set_label("Preparing…");
    pp.cover.set_paintable(None::<&gtk::gdk::Paintable>);
    while let Some(r) = pp.chapter_list.first_child() { pp.chapter_list.remove(&r); }
    if app.nav.visible_page().is_none_or(|p| p.tag().as_deref() != Some("player")) { app.nav.push(&pp.page); }

    if let Some(url) = item.product_images.get("500").cloned() {
        let asin = item.asin.clone();
        glib::spawn_future_local(glib::clone!(#[strong] app, async move {
            if let Ok(p) = crate::rt::io(async move { covers::ensure(&asin, &url).await }).await { app.player_page.cover.set_filename(Some(&p)); }
        }));
    }

    let asin = item.asin.clone();
    let (tx, rx) = async_channel::unbounded::<(u64, Option<u64>)>();
    glib::spawn_future_local(glib::clone!(#[strong] app, async move {
        while let Ok((done, total)) = rx.recv().await {
            let s = match total { Some(t) => format!("Downloading… {}%", done * 100 / t.max(1)), None => format!("Downloading… {} MB", done >> 20) };
            app.player_page.status.set_label(&s);
        }
    }));
    if let Some(h) = app.download.borrow_mut().take() { h.abort(); }
    let handle = crate::rt::rt().spawn(async move {
        cache::ensure(&client, &asin, move |d, t| { let _ = tx.try_send((d, t)); }).await
    });
    *app.download.borrow_mut() = Some(handle.abort_handle());
    glib::spawn_future_local(glib::clone!(#[strong] app, async move {
        let res = match handle.await {
            Ok(r) => r,
            Err(_) => return, // aborted: another book was opened
        };
        *app.download.borrow_mut() = None;
        match res {
            Ok((path, lic)) => {
                let start = lic.content_metadata.last_position_heard.as_ref().map(|p| p.position_ms).unwrap_or(0);
                let chapters = chapters::from_license(&lic.content_metadata.chapter_info);
                for c in &chapters {
                    let row = adw::ActionRow::builder().title(&c.title).subtitle(fmt_ms(c.length_ms)).activatable(true).build();
                    app.player_page.chapter_list.append(&row);
                }
                *app.current.borrow_mut() = Some(Current { item: item.clone(), acr: lic.acr.clone(), chapters });
                app.sync.reset();
                app.player_page.status.set_label("");
                // Play at once, silent when there is a position to resume, and
                // seek there once it is playing (`poll`); the volume comes back
                // when the seek has landed (`on_position`). Seeking while paused
                // left the PipeWire sink never prerolling again, so nothing played.
                debug(|| format!("open {}: resume at {start} ms", item.asin));
                app.resume.set(Resume::Prerolling(start));
                app.engine.load(&path);
                if start > 0 { app.engine.set_volume(0.0); }
                app.engine.play();
                crate::player::mpris::set_track(&app, &item, app.engine.duration_ms());
            }
            Err(e) => { app.player_page.status.set_label(""); toast(&app, &format!("could not open book: {e:#}")); }
        }
    }));
}

/// Delete the open book's download and go back to the library.
fn remove_download(a: &AppRef) {
    if let Some(h) = a.download.borrow_mut().take() { h.abort(); }
    a.engine.pause();
    let asin = a.current.borrow_mut().take().map(|c| c.item.asin);
    if let Some(asin) = asin {
        a.engine.stop();
        let _ = std::fs::remove_file(cache::m4b_path(&asin));
        for p in cache::scratch_paths(&asin) { let _ = std::fs::remove_file(p); }
        toast(a, "Download removed");
    }
    a.nav.pop_to_tag("library");
    super::library::render(a);
}

/// The playback speeds, in a centred modal (a tap outside keeps the speed).
fn choose_speed(app: &AppRef) {
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
    let content = adw::Clamp::builder().child(&list).margin_top(6).margin_bottom(18).margin_start(12).margin_end(12).build();
    let dialog = omarchy_mobile::modal::dialog("Speed", &content);
    for (i, s) in app.player_page.speeds.iter().enumerate() {
        let row = adw::ActionRow::builder().title(format!("{s}×")).activatable(true).build();
        if i == app.player_page.speed_index.get() {
            row.add_suffix(&gtk::Image::from_icon_name("object-select-symbolic"));
        }
        row.connect_activated(glib::clone!(#[strong] app, #[weak] dialog, move |_| {
            set_speed(&app, i);
            dialog.close();
        }));
        list.append(&row);
    }
    omarchy_mobile::modal::present(&dialog, Some(&app.window));
}

fn set_speed(app: &AppRef, i: usize) {
    let rate = app.player_page.speeds[i];
    app.player_page.speed_index.set(i);
    app.player_page.speed.set_label(&format!("{rate}×"));
    app.engine.set_rate(rate);
    crate::player::mpris::set_rate(app, rate);
}

pub fn toggle_play(app: &AppRef) {
    if app.current.borrow().is_none() { return; }
    if app.engine.is_playing() { app.engine.pause(); } else { app.engine.play(); }
}

fn jump_chapter(app: &AppRef, delta: i32) {
    let Some(pos) = app.engine.position_ms() else { return };
    let target = {
        let cur = app.current.borrow();
        let Some(c) = cur.as_ref() else { return };
        let idx = chapters::index_at(&c.chapters, pos).unwrap_or(0) as i32;
        // "prev" within the first 3s of a chapter goes to the previous one; otherwise restarts it.
        let idx = if delta < 0 && c.chapters.get(idx as usize).is_some_and(|ch| pos > ch.start_offset_ms + 3000) { idx } else { idx + delta };
        c.chapters.get(idx.clamp(0, c.chapters.len() as i32 - 1) as usize).map(|ch| ch.start_offset_ms)
    };
    if let Some(ms) = target { app.engine.seek_ms(ms); after_seek(app); }
}

fn after_seek(app: &AppRef) {
    glib::timeout_add_local_once(std::time::Duration::from_millis(300), glib::clone!(#[strong] app, move || {
        if let Some(ms) = app.engine.position_ms() { on_position(&app, ms); push_position(&app, ms, true); crate::player::mpris::seeked(&app, ms); }
    }));
}

fn on_position(app: &AppRef, ms: u64) {
    // Arm pushes only once the resume-seek has actually landed. If it silently
    // failed we stay unarmed and sync nothing, which is the safe way to fail:
    // pushing the resulting ~0 would overwrite the server's real position.
    if let Resume::Seeking(start) = app.resume.get() {
        if ms + LANDED_TOLERANCE_MS < start { return; }
        debug(|| format!("resume landed at {ms} ms"));
        app.resume.set(Resume::Armed);
        app.engine.set_volume(1.0);
        app.sync.mark(ms);
    }
    let pp = &app.player_page;
    if !pp.seeking.get() { pp.scale.set_value(ms as f64); update_time_labels(app, ms); }
    let idx = app.current.borrow().as_ref().and_then(|c| chapters::index_at(&c.chapters, ms).map(|i| (i, c.chapters[i].title.clone())));
    if let Some((i, title)) = idx
        && pp.chapter.label() != title
    {
        pp.chapter.set_label(&title);
        if let Some(row) = pp.chapter_list.row_at_index(i as i32) { pp.chapter_list.select_row(Some(&row)); }
    }
    crate::player::mpris::set_position(app, ms);
    if app.engine.is_playing() { push_position(app, ms, false); }
}

fn update_time_labels(app: &App, ms: u64) {
    let pp = &app.player_page;
    let dur = app.engine.duration_ms().unwrap_or(0);
    pp.elapsed.set_label(&fmt_ms(ms));
    pp.remaining.set_label(&format!("-{}", fmt_ms(dur.saturating_sub(ms))));
}

fn on_state(app: &AppRef, state: gst_play::PlayState) {
    let playing = state == gst_play::PlayState::Playing;
    app.player_page.play_btn.set_icon_name(if playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" });
    crate::player::mpris::set_playing(app, playing);
    if state == gst_play::PlayState::Paused && app.resume.get() == Resume::Armed {
        if let Some(ms) = app.engine.position_ms() { push_position(app, ms, true); }
    }
}

/// Throttled Whispersync push (see `SyncState`). `force` for pause/seek/quit.
pub fn push_position(app: &AppRef, ms: u64, force: bool) {
    if app.resume.get() != Resume::Armed || !app.sync.should_push(ms, force) { return; }
    let (Some(client), Some((asin, acr))) = (app.client.borrow().clone(), app.current.borrow().as_ref().map(|c| (c.item.asin.clone(), c.acr.clone()))) else { return };
    app.sync.mark(ms);
    {
        // Keep the library view's notion of progress current without a refetch.
        let mut ps = app.positions.borrow_mut();
        let now = glib::DateTime::now_utc().ok().and_then(|d| d.format("%Y-%m-%d %H:%M:%S").ok()).map(|s| s.to_string());
        match ps.iter_mut().find(|p| p.asin.as_deref() == Some(&asin)) {
            Some(p) => { p.position_ms = ms; p.last_updated = now; }
            None => ps.push(positions::LastPosition { asin: Some(asin.clone()), position_ms: ms, last_updated: now, status: Some("Exists".into()) }),
        }
    }
    glib::spawn_future_local(glib::clone!(#[strong] app, async move {
        let log_asin = asin.clone();
        if let Err(e) = crate::rt::io(async move { positions::write(&client, &asin, &acr, ms).await }).await {
            toast(&app, &format!("sync failed: {e:#}"));
        } else {
            tracing::info!(asin = log_asin, ms, "synced");
        }
    }));
}

/// Synchronous final push on window close (≤3s).
pub fn push_position_blocking(app: &Rc<App>, ms: u64) {
    if app.resume.get() != Resume::Armed { return; }
    let (Some(client), Some((asin, acr))) = (app.client.borrow().clone(), app.current.borrow().as_ref().map(|c| (c.item.asin.clone(), c.acr.clone()))) else { return };
    let _ = crate::rt::rt().block_on(async move {
        tokio::time::timeout(std::time::Duration::from_secs(3), positions::write(&client, &asin, &acr, ms)).await
    });
}
