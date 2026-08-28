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

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new("Sign in to Audible", "")));
    let names: Vec<&str> = LOCALES.iter().map(|l| l.1).collect();
    let picker = gtk::DropDown::from_strings(&names);
    picker.set_tooltip_text(Some("Audible marketplace"));
    picker.connect_selected_notify(glib::clone!(#[weak] web, #[strong] pending, move |dd| {
        let p = PendingLogin::start(LOCALES[dd.selected() as usize].0);
        web.load_uri(&p.url);
        *pending.borrow_mut() = Some(p);
    }));
    header.pack_end(&picker);
    let tv = adw::ToolbarView::builder().content(&web).build();
    tv.add_top_bar(&header);
    let page = adw::NavigationPage::builder().title("Sign in").child(&tv).tag("login").can_pop(false).build();

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
