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
    /// Waiting for the pipeline to preroll (`PlayState::Paused`) before seeking here.
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
    /// Keeps the live Omarchy theme monitor and CSS provider alive.
    pub _omarchy_theme: Option<crate::omarchy::OmarchyTheme>,
}

pub type AppRef = Rc<App>;

/// Skald's own stylesheet. It sits below the accent fallback and the Omarchy
/// theme, so both can override what it sets.
const CSS: &str = "
/* Sidebar opacity: --skald-sidebar-opacity (a number, 0 to 1) fades the chapter
   sidebar's background but never its text. An Omarchy theme's skald.css sets
   it. Below 1, the window also gets .skald-transparent (see build) so the
   desktop shows through instead of the window background. Scoped to the
   split view's own pane so nested panes, which Adwaita keeps transparent,
   are left alone. */
:root { --skald-sidebar-opacity: 1; }
overlay-split-view > .sidebar-pane {
    background-color: alpha(var(--sidebar-bg-color), var(--skald-sidebar-opacity));
}
overlay-split-view > .sidebar-pane:backdrop {
    background-color: alpha(var(--sidebar-backdrop-color), var(--skald-sidebar-opacity));
}
window.skald-transparent { background-color: transparent; }
/* Everything else drew on the window background removed above, so paint it
   back. Library and sign-in pages (grid, status pages, web view) are opaque
   whole. On the player page, only the header and the split view's content
   side are: the page itself stays clear behind the sidebar. Paint the header
   on the .top-bar, NOT on the headerbar: GTK dims the windowhandle subtree to
   50% on backdrop, which would fade the headerbar into the wallpaper. The
   split view never collapses (it has no breakpoint), so its content always
   carries .content-pane. Popovers, dialogs and toasts paint their own
   backgrounds. */
window.skald-transparent navigation-view-page:not(.skald-player),
window.skald-transparent navigation-view-page.skald-player toolbarview > .top-bar,
window.skald-transparent overlay-split-view > .content-pane {
    background-color: var(--window-bg-color);
}
";

pub fn build(gtk_app: &adw::Application) {
    let omarchy_theme = gtk::gdk::Display::default().map(|display| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(CSS);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        crate::accent::install_fallback(&display);
        crate::omarchy::OmarchyTheme::install(&display)
    });
    let open_asin = std::env::args().nth(1).filter(|a| a == "open").and_then(|_| std::env::args().nth(2));
    let nav = adw::NavigationView::new();
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&nav));
    let window = adw::ApplicationWindow::builder()
        .application(gtk_app).title("Skald")
        .default_width(1000).default_height(700)
        .content(&toasts).build();
    // A translucent sidebar only reaches the desktop once the window stops
    // painting its own background underneath.
    if let Some(theme) = &omarchy_theme {
        theme.connect_sidebar_opacity(glib::clone!(
            #[weak]
            window,
            move |opacity| {
                if opacity < 1.0 {
                    window.add_css_class("skald-transparent")
                } else {
                    window.remove_css_class("skald-transparent")
                }
            }
        ));
    }

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
        _omarchy_theme: omarchy_theme,
    });

    library::attach(&app);
    player::attach(&app);
    crate::player::mpris::attach(&app);

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

    match crate::auth::store::AuthState::load() {
        Ok(Some(state)) => match Client::new(state) {
            Ok(c) => { *app.client.borrow_mut() = Some(c); library::refresh(&app); if let Some(asin) = open_asin { library::open_when_loaded(&app, asin); } }
            Err(e) => toast(&app, &format!("auth error: {e}")),
        },
        _ => login::show(&app),
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
