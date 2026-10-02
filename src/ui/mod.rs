pub mod library;
pub mod login;
pub mod player;

use crate::api::{chapters::Chapter, client::Client, library::LibraryItem, positions::LastPosition};
use crate::player::{engine::Engine, sync::SyncState};
use adw::prelude::*;
use gtk4 as gtk;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

/// Resume-seek state machine. A position is pushed to the server only in `Armed`,
/// so a seek that hasn't landed yet can never clobber the position the phone set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resume {
    /// No book loaded, or the load failed.
    Idle,
    /// Started (muted when there is a position to resume); waiting for it to be
    /// playing before seeking here.
    Prerolling(u64),
    /// Seek issued; waiting for the reported position to actually reach it.
    Seeking(u64),
    /// Playing at the resumed position. Pushes allowed.
    Armed,
}

/// Currently loaded book.
pub struct Current {
    pub item: LibraryItem,
    pub acr: String,
    pub chapters: Vec<Chapter>,
}

pub struct App {
    pub window: adw::ApplicationWindow,
    pub nav: adw::NavigationView,
    pub toasts: adw::ToastOverlay,
    pub client: RefCell<Option<Client>>,
    pub engine: Engine,
    pub current: RefCell<Option<Current>>,
    pub sync: SyncState,
    /// Reset by `open()`; no position pushes until the resume-seek has landed.
    pub resume: std::cell::Cell<Resume>,
    /// In-flight download for the book being opened; aborted if the user opens another book.
    pub download: RefCell<Option<tokio::task::AbortHandle>>,
    pub library: RefCell<Vec<LibraryItem>>,
    pub positions: RefCell<Vec<LastPosition>>,
    pub player_page: player::PlayerPage,
    pub library_page: library::LibraryPage,
    pub mpris: RefCell<Option<Rc<mpris_server::Player>>>,
}

pub type AppRef = Rc<App>;

pub fn build(gtk_app: &adw::Application) {
    // Dev hook: SKALD_OPEN=<ASIN> opens that book once the library is in.
    let open_asin = std::env::var("SKALD_OPEN").ok().filter(|a| !a.is_empty());
    let nav = adw::NavigationView::new();
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&nav));
    // The phone's window (omarchy-mobile kit): pages carry the title only;
    // back is the shell's edge swipe, the menu its home bar.
    let window = omarchy_mobile::window(gtk_app, "Skald", &toasts);

    let app: AppRef = Rc::new(App {
        window: window.clone(), nav: nav.clone(), toasts,
        client: RefCell::new(None),
        engine: Engine::new(),
        current: RefCell::new(None),
        sync: SyncState::default(),
        resume: std::cell::Cell::new(Resume::Idle),
        download: RefCell::new(None),
        library: RefCell::new(Vec::new()),
        positions: RefCell::new(Vec::new()),
        player_page: player::PlayerPage::new(),
        library_page: library::LibraryPage::new(),
        mpris: RefCell::new(None),
    });

    library::attach(&app);
    player::attach(&app);
    crate::player::mpris::attach(&app);
    omarchy_mobile::connect_back(gtk_app, &nav);
    omarchy_mobile::set_menu(
        gtk_app,
        &[&[("Refresh", "app.refresh"), ("Clear downloads", "app.clear-downloads")], &[("Sign out", "app.sign-out"), ("About Skald", "app.about")]],
        None,
    );

    // Push final position on close; block briefly so the request gets out.
    window.connect_close_request(glib::clone!(#[strong] app, move |_| {
        app.engine.pause();
        if let Some(ms) = app.engine.position_ms() { player::push_position_blocking(&app, ms); }
        glib::Propagation::Proceed
    }));

    // SIGTERM/SIGINT (logout, kill) → same path as closing the window, so the position is flushed.
    let (tx, rx) = async_channel::bounded::<()>(1);
    crate::rt::rt().spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let (Ok(mut term), Ok(mut int)) = (signal(SignalKind::terminate()), signal(SignalKind::interrupt())) else { return };
        tokio::select! { _ = term.recv() => {}, _ = int.recv() => {} }
        let _ = tx.send(()).await;
    });
    glib::spawn_future_local(glib::clone!(#[weak] window, async move {
        if rx.recv().await.is_ok() { window.close(); }
    }));

    nav.push(&app.library_page.page);
    window.present();

    // From the keyring: a moment's D-Bus round trip, before the first page.
    match crate::rt::rt().block_on(crate::auth::store::AuthState::load()) {
        Ok(Some(state)) => match Client::new(state) {
            Ok(c) => { *app.client.borrow_mut() = Some(c); library::refresh(&app); if let Some(asin) = open_asin { library::open_when_loaded(&app, asin); } }
            Err(e) => toast(&app, &format!("auth error: {e}")),
        },
        _ => login::show(&app),
    }
}

/// A destructive action, confirmed in a centred modal first (omarchy-mobile's
/// app guidelines: destructive actions ask, and a tap outside cancels).
pub fn confirm(app: &AppRef, title: &str, body: &str, verb: &str, then: fn(&AppRef)) {
    let dialog = adw::AlertDialog::new(Some(title), Some(body));
    dialog.add_responses(&[("cancel", "Cancel"), ("do", verb)]);
    dialog.set_response_appearance("do", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    dialog.connect_response(Some("do"), glib::clone!(#[strong] app, move |_, _| then(&app)));
    omarchy_mobile::modal::present(&dialog, Some(&app.window));
}

/// `SKALD_DEBUG=1`: the player's messages and resume steps on stderr.
pub fn debug(what: impl FnOnce() -> String) {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("SKALD_DEBUG").is_some()) {
        eprintln!("skald: {}", what());
    }
}

pub fn toast(app: &App, msg: &str) {
    tracing::info!("{msg}");
    app.toasts.add_toast(adw::Toast::new(msg));
}

pub fn fmt_ms(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, s) = (s / 3600, s % 3600 / 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}
