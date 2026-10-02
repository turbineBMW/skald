//! One-time sign-in: Amazon's real login page in a WebView; the `/ap/maplanding`
//! redirect carries the authorization code, which we exchange for device tokens.
use super::{AppRef, toast};
use crate::auth::login::{PendingLogin, code_from_redirect};
use adw::prelude::*;
use gtk4 as gtk;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;

const LOCALES: &[(&str, &str)] = &[("us", "United States"), ("uk", "United Kingdom"), ("ca", "Canada"), ("au", "Australia"),
    ("de", "Germany"), ("fr", "France"), ("it", "Italy"), ("es", "Spain"), ("in", "India"), ("jp", "Japan"), ("br", "Brazil")];

pub fn show(app: &AppRef) {
    let pending = Rc::new(RefCell::new(Some(PendingLogin::start("us"))));
    let url = pending.borrow().as_ref().unwrap().url.clone();

    let web = webkit6::WebView::new();
    if let Some(s) = WebViewExt::settings(&web) {
        s.set_user_agent(Some("Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.0 Mobile/15E148 Safari/604.1"));
    }
    web.set_vexpand(true);

    // The title only at the top; the marketplace in the action row, as a
    // modal list (omarchy-mobile's app guidelines).
    let page = omarchy_mobile::page("Sign in to Audible", "login", &web);
    page.set_can_pop(false);
    let actions = omarchy_mobile::Actions::new();
    let market = gtk::Button::with_label(LOCALES[0].1);
    market.connect_clicked(glib::clone!(#[strong] app, #[weak] web, #[strong] pending, move |button| {
        choose_marketplace(&app, button, &web, &pending);
    }));
    actions.add_button(&market);
    omarchy_mobile::set_actions(&page, &actions);

    web.connect_decide_policy(glib::clone!(#[strong] app, #[strong] pending, move |_wv, decision, kind| {
        if kind != webkit6::PolicyDecisionType::NavigationAction { return false; }
        let Some(nav) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else { return false };
        let Some(uri) = nav.navigation_action().and_then(|a| a.request()).and_then(|r| r.uri()) else { return false };
        if !uri.contains("/ap/maplanding") { return false; }
        decision.ignore();
        let Some(code) = code_from_redirect(&uri) else { toast(&app, "sign-in redirect had no code"); return true };
        let Some(p) = pending.borrow_mut().take() else { return true };
        glib::spawn_future_local(glib::clone!(#[strong] app, async move {
            match crate::rt::io(async move { p.register(&code).await }).await {
                Ok(state) => {
                    if let Err(e) = state.save() { toast(&app, &format!("could not save auth: {e}")); }
                    match crate::api::client::Client::new(state) {
                        Ok(c) => {
                            *app.client.borrow_mut() = Some(c);
                            app.nav.pop_to_tag("library");
                            super::library::refresh(&app);
                        }
                        Err(e) => toast(&app, &format!("auth error: {e}")),
                    }
                }
                Err(e) => toast(&app, &format!("registration failed: {e:#}")),
            }
        }));
        true
    }));

    web.load_uri(&url);
    app.nav.push(&page);
}

/// The Audible marketplaces in a centred modal; picking one starts the sign-in
/// again there.
fn choose_marketplace(app: &AppRef, button: &gtk::Button, web: &webkit6::WebView, pending: &Rc<RefCell<Option<PendingLogin>>>) {
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
    let content = adw::Clamp::builder().child(&list).margin_top(6).margin_bottom(18).margin_start(12).margin_end(12).build();
    let scroll = gtk::ScrolledWindow::builder().child(&content).hscrollbar_policy(gtk::PolicyType::Never).propagate_natural_height(true).build();
    let dialog = omarchy_mobile::modal::dialog("Marketplace", &scroll);
    for (code, name) in LOCALES {
        let row = adw::ActionRow::builder().title(*name).activatable(true).build();
        row.connect_activated(glib::clone!(#[weak] web, #[weak] button, #[weak] dialog, #[strong] pending, move |_| {
            let p = PendingLogin::start(code);
            web.load_uri(&p.url);
            *pending.borrow_mut() = Some(p);
            button.set_label(name);
            dialog.close();
        }));
        list.append(&row);
    }
    omarchy_mobile::modal::present_popup(&dialog, Some(&app.window));
}
