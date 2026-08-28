use super::{AppRef, fmt_ms, toast};
use crate::api::{covers, library::LibraryItem, positions};
use adw::prelude::*;
use gtk4 as gtk;
use gtk::glib;

pub struct LibraryPage {
    pub page: adw::NavigationPage,
    pub flow: gtk::FlowBox,
    pub recent_section: gtk::Box,
    pub recent: gtk::Box,
    pub spinner: adw::Spinner,
    pub search: gtk::SearchEntry,
}

impl LibraryPage {
    pub fn new() -> Self {
        let flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .homogeneous(true).column_spacing(18).row_spacing(18)
            .margin_start(24).margin_end(24).margin_top(12).margin_bottom(24)
            .min_children_per_line(2).max_children_per_line(8).build();
        let section_label = |t: &str| gtk::Label::builder().label(t).xalign(0.0).css_classes(["title-3"]).margin_start(24).margin_top(18).build();
        let recent = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(18).margin_start(24).margin_end(24).margin_top(6).build();
        let recent_scroll = gtk::ScrolledWindow::builder().child(&recent).vscrollbar_policy(gtk::PolicyType::Never)
            .hscrollbar_policy(gtk::PolicyType::Automatic).propagate_natural_height(true).build();
        let recent_section = gtk::Box::new(gtk::Orientation::Vertical, 0);
        recent_section.append(&section_label("Continue listening"));
        recent_section.append(&recent_scroll);
        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.append(&recent_section);
        column.append(&section_label("Library"));
        column.append(&flow);
        let scroll = gtk::ScrolledWindow::builder().child(&column).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
        let spinner = adw::Spinner::new();
        let search = gtk::SearchEntry::builder().placeholder_text("Search library").width_chars(28).build();
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&search));
        header.pack_end(&spinner);
        let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_action_name(Some("app.refresh"));
        refresh_btn.set_tooltip_text(Some("Refresh library"));
        header.pack_start(&refresh_btn);
        let menu = gtk::gio::Menu::new();
        menu.append(Some("Clear downloads"), Some("app.clear-downloads"));
        menu.append(Some("Sign out"), Some("app.sign-out"));
        menu.append(Some("About Skald"), Some("app.about"));
        header.pack_end(&gtk::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&menu).primary(true).build());
        let tv = adw::ToolbarView::builder().content(&scroll).build();
        tv.add_top_bar(&header);
        let page = adw::NavigationPage::builder().title("Library").tag("library").child(&tv).build();
        Self { page, flow, recent_section, recent, spinner, search }
    }
}

pub fn attach(app: &AppRef) {
    let gtk_app = app.window.application().unwrap();
    let add = |name: &str, f: Box<dyn Fn(&AppRef)>| {
        let a = gtk::gio::SimpleAction::new(name, None);
        a.connect_activate(glib::clone!(#[strong] app, move |_, _| f(&app)));
        gtk_app.add_action(&a);
    };
    add("refresh", Box::new(refresh));
    add("about", Box::new(|a| {
        adw::AboutDialog::builder().application_name("Skald").application_icon("dev.turbinebmw.Skald")
            .version(env!("CARGO_PKG_VERSION")).developer_name("turbinebmw")
            .comments("A native Audible client that keeps Whispersync working.").build().present(Some(&a.window));
    }));
    add("sign-out", Box::new(|a| {
        a.engine.pause();
        *a.client.borrow_mut() = None;
        *a.current.borrow_mut() = None;
        let _ = std::fs::remove_file(crate::auth::store::path());
        while let Some(c) = a.library_page.flow.first_child() { a.library_page.flow.remove(&c); }
        a.nav.pop_to_tag("library");
        super::login::show(a);
    }));
    add("clear-downloads", Box::new(|a| {
        a.engine.pause();
        *a.current.borrow_mut() = None;
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(crate::player::cache::dir()) {
            for e in rd.flatten() {
                if e.path().extension().is_some_and(|x| x == "m4b" || x == "aaxc") && std::fs::remove_file(e.path()).is_ok() { n += 1; }
            }
        }
        toast(a, &format!("Removed {n} downloaded book{}", if n == 1 { "" } else { "s" }));
        a.nav.pop_to_tag("library");
        render(a);
    }));

    app.library_page.search.connect_search_changed(glib::clone!(#[strong] app, move |e| {
        let q = e.text().to_lowercase();
        app.library_page.recent_section.set_visible(q.is_empty());
        let flow = &app.library_page.flow;
        let mut i = 0;
        while let Some(child) = flow.child_at_index(i) {
            let hay = child.widget_name().to_lowercase();
            child.set_visible(q.is_empty() || hay.contains(&q));
            i += 1;
        }
    }));

    // Cards in the Continue-listening row are plain buttons (not FlowBox children).
    let _ = &app.library_page.recent;
    app.library_page.flow.connect_child_activated(glib::clone!(#[strong] app, move |_, child| {
        let asin = child.tooltip_text().map(|s| s.to_string()).unwrap_or_default();
        let item = app.library.borrow().iter().find(|i| i.asin == asin).cloned();
        if let Some(item) = item { super::player::open(&app, item); }
    }));
}

/// Dev helper (`skald open ASIN`): open a book as soon as the library has loaded.
pub fn open_when_loaded(app: &AppRef, asin: String) {
    glib::timeout_add_local(std::time::Duration::from_millis(200), glib::clone!(#[strong] app, move || {
        let item = app.library.borrow().iter().find(|i| i.asin == asin).cloned();
        match item { Some(it) => { super::player::open(&app, it); glib::ControlFlow::Break } None => glib::ControlFlow::Continue }
    }));
}

pub fn refresh(app: &AppRef) {
    let Some(client) = app.client.borrow().clone() else { return };
    app.library_page.spinner.set_visible(true);
    glib::spawn_future_local(glib::clone!(#[strong] app, async move {
        let res = crate::rt::io(async move {
            let lib = crate::api::library::fetch_all(&client).await?;
            let asins: Vec<&str> = lib.iter().map(|i| i.asin.as_str()).collect();
            let pos = positions::read(&client, &asins).await?;
            anyhow::Ok((lib, pos))
        }).await;
        app.library_page.spinner.set_visible(false);
        match res {
            Ok((lib, pos)) => {
                *app.library.borrow_mut() = lib;
                *app.positions.borrow_mut() = pos;
                render(&app);
            }
            Err(e) => toast(&app, &format!("library: {e:#}")),
        }
    }));
}

pub fn render(app: &AppRef) {
    let flow = &app.library_page.flow;
    while let Some(c) = flow.first_child() { flow.remove(&c); }
    let positions = app.positions.borrow();
    let mut items: Vec<LibraryItem> = app.library.borrow().clone();
    // Recently-listened first, then the rest in library order.
    let key = |i: &LibraryItem| positions.iter().find(|p| p.asin.as_deref() == Some(&i.asin))
        .and_then(|p| p.last_updated.clone()).unwrap_or_default();
    items.sort_by_key(|i| std::cmp::Reverse(key(i)));

    let recent = &app.library_page.recent;
    while let Some(c) = recent.first_child() { recent.remove(&c); }
    let pos_of = |item: &LibraryItem| positions.iter().find(|p| p.asin.as_deref() == Some(&item.asin)).map(|p| p.position_ms).unwrap_or(0);
    // In-progress = has a position and isn't within the last minute of the book.
    let in_progress: Vec<&LibraryItem> = items.iter().filter(|i| {
        let p = pos_of(i); let total = i.runtime_length_min.unwrap_or(0) * 60_000;
        p > 0 && (total == 0 || p + 60_000 < total)
    }).take(10).collect();
    app.library_page.recent_section.set_visible(!in_progress.is_empty());
    for item in in_progress {
        let card = build_card(app, item, pos_of(item));
        let inner = card.child().unwrap();
        card.set_child(None::<&gtk::Widget>);
        let btn = gtk::Button::builder().child(&inner).css_classes(["flat"]).build();
        let it = item.clone();
        btn.connect_clicked(glib::clone!(#[strong] app, move |_| super::player::open(&app, it.clone())));
        recent.append(&btn);
    }

    for item in items {
        let pos = pos_of(&item);
        let card = build_card(app, &item, pos);
        flow.insert(&card, -1);
    }
}

fn build_card(app: &AppRef, item: &LibraryItem, position_ms: u64) -> gtk::FlowBoxChild {
    let pic = gtk::Picture::builder().width_request(160).height_request(160)
        .content_fit(gtk::ContentFit::Cover).can_shrink(false).halign(gtk::Align::Center).css_classes(["card"]).build();
    let overlay = gtk::Overlay::builder().child(&pic).build();
    if crate::player::cache::m4b_path(&item.asin).exists() {
        let badge = gtk::Image::builder().icon_name("folder-download-symbolic").pixel_size(14)
            .tooltip_text("Downloaded").halign(gtk::Align::End).valign(gtk::Align::Start)
            .margin_end(6).margin_top(6).css_classes(["osd", "circular"]).build();
        badge.set_size_request(24, 24);
        overlay.add_overlay(&badge);
    }
    let title = gtk::Label::builder().label(&item.title).wrap(true).lines(2).ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(20).justify(gtk::Justification::Center).css_classes(["heading"]).build();
    let author = item.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ");
    let sub = gtk::Label::builder().label(&author).ellipsize(gtk::pango::EllipsizeMode::End).max_width_chars(20)
        .css_classes(["dim-label", "caption"]).build();
    let total = item.runtime_length_min.unwrap_or(0) * 60_000;
    let bar = gtk::ProgressBar::builder().fraction(if total > 0 { (position_ms as f64 / total as f64).min(1.0) } else { 0.0 })
        .visible(position_ms > 0).build();
    let prog = gtk::Label::builder().label(if position_ms > 0 { format!("{} / {}", fmt_ms(position_ms), fmt_ms(total)) } else { String::new() })
        .css_classes(["dim-label", "caption"]).build();
    let bx = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(4).width_request(160).build();
    for w in [overlay.upcast_ref::<gtk::Widget>(), title.upcast_ref(), sub.upcast_ref(), bar.upcast_ref(), prog.upcast_ref()] { bx.append(w); }
    let child = gtk::FlowBoxChild::builder().child(&bx).tooltip_text(&item.asin)
        .name(format!("{} {}", item.title, author)).build();

    if let Some(url) = item.product_images.get("500").cloned() {
        let asin = item.asin.clone();
        glib::spawn_future_local(glib::clone!(#[weak] pic, #[strong] app, async move {
            match crate::rt::io(async move { covers::ensure(&asin, &url).await }).await {
                Ok(p) => pic.set_filename(Some(&p)),
                Err(e) => tracing::debug!("cover: {e}"),
            }
            let _ = &app;
        }));
    }
    child
}
