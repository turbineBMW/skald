//! Follow the active Omarchy desktop palette and reload it live.
//!
//! A theme may also ship its own `skald.css`, either by hand or generated from
//! `~/.config/omarchy/themed/skald.css.tpl`. It is appended after the CSS
//! derived from `colors.toml`, so whatever it states wins and whatever it
//! leaves out keeps the palette's value: a file can restate every color, or
//! carry a single line such as `:root { --skald-sidebar-opacity: 0.8; }`.

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const SETTLE: Duration = Duration::from_millis(120);

type OpacityCallback = Box<dyn Fn(f32)>;

/// How opaque the chapter sidebar's background is, 0 to 1. Skald's built-in
/// stylesheet defaults it to 1; only a theme's `skald.css` changes it.
pub const SIDEBAR_OPACITY: &str = "--skald-sidebar-opacity";

/// Owns the CSS provider and file monitor for the lifetime of the app.
pub struct OmarchyTheme {
    inner: Rc<Inner>,
}

struct Inner {
    provider: gtk::CssProvider,
    monitor: RefCell<Option<gio::FileMonitor>>,
    /// `--skald-sidebar-opacity` as the active theme sets it, 1 when it
    /// doesn't. Worked out on every reload.
    sidebar_opacity: Cell<f32>,
    on_sidebar_opacity: RefCell<Option<OpacityCallback>>,
}

impl OmarchyTheme {
    /// Install above the system-accent fallback so the desktop palette wins.
    pub fn install(display: &gtk::gdk::Display) -> Self {
        let inner = Rc::new(Inner {
            provider: gtk::CssProvider::new(),
            monitor: RefCell::new(None),
            sidebar_opacity: Cell::new(1.0),
            on_sidebar_opacity: RefCell::new(None),
        });
        gtk::style_context_add_provider_for_display(
            display,
            &inner.provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
        );
        inner.provider.connect_parsing_error(|_, section, error| {
            tracing::warn!(
                "Omarchy theme CSS line {}: {error}",
                section.start_location().lines() + 1
            );
        });
        inner.watch();
        inner.reload();
        Self { inner }
    }

    /// Call `f` with the theme's sidebar opacity now and again whenever a
    /// theme change alters it. The window needs it to know when to clear its
    /// own background, which GTK cannot report for a CSS custom property.
    pub fn connect_sidebar_opacity(&self, f: impl Fn(f32) + 'static) {
        f(self.inner.sidebar_opacity.get());
        self.inner.on_sidebar_opacity.replace(Some(Box::new(f)));
    }
}

impl Inner {
    fn reload(&self) {
        let theme = load(&glib::home_dir());
        self.provider
            .load_from_string(theme.as_ref().map_or("", |theme| theme.css.as_str()));
        // This provider sits above Skald's built-in default of 1, so the
        // theme's value is the one GTK uses whenever it states one.
        let opacity = theme
            .as_ref()
            .and_then(|theme| opacity_in(&theme.css, SIDEBAR_OPACITY))
            .unwrap_or(1.0);
        adw::StyleManager::default().set_color_scheme(match theme {
            Some(theme) if theme.light => adw::ColorScheme::ForceLight,
            Some(_) => adw::ColorScheme::ForceDark,
            None => adw::ColorScheme::Default,
        });
        if self.sidebar_opacity.replace(opacity) != opacity
            && let Some(f) = self.on_sidebar_opacity.borrow().as_ref()
        {
            f(opacity);
        }
    }

    /// Omarchy replaces the whole theme directory, so watch its stable parent.
    fn watch(self: &Rc<Self>) {
        let dir = state_dir(&glib::home_dir());
        if !dir.is_dir() {
            return;
        }
        let monitor = match gio::File::for_path(&dir)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            Ok(monitor) => monitor,
            Err(error) => {
                tracing::warn!("cannot watch {}: {error}", dir.display());
                return;
            }
        };
        let pending = Rc::new(Cell::new(false));
        monitor.connect_changed(glib::clone!(
            #[weak(rename_to = inner)]
            self,
            move |_, _, _, _| {
                if pending.replace(true) {
                    return;
                }
                let pending = pending.clone();
                glib::timeout_add_local_once(
                    SETTLE,
                    glib::clone!(
                        #[weak]
                        inner,
                        move || {
                            pending.set(false);
                            inner.reload();
                        }
                    ),
                );
            }
        ));
        self.monitor.replace(Some(monitor));
    }
}

fn state_dir(home: &Path) -> PathBuf {
    home.join(".local/state/omarchy/current")
}

fn theme_dir(home: &Path) -> PathBuf {
    state_dir(home).join("theme")
}

struct Theme {
    css: String,
    light: bool,
}

fn load(home: &Path) -> Option<Theme> {
    load_in(&theme_dir(home))
}

fn load_in(dir: &Path) -> Option<Theme> {
    let colors = fs::read_to_string(dir.join("colors.toml")).ok()?;
    let palette = Palette::resolve(&colors, dir.join("light.mode").exists());
    // A theme may provide app-specific CSS, just as it can for Rustle. It is
    // layered over the derived palette rather than replacing it, so a partial
    // file only changes what it names.
    let mut css = palette.to_css();
    if let Ok(own) = fs::read_to_string(dir.join("skald.css")) {
        css.push_str("/* The theme's own skald.css. */\n");
        css.push_str(&own);
    }
    Some(Theme {
        css,
        light: palette.light,
    })
}

/// The last value `name` is given anywhere in `css`, as an opacity clamped
/// to 0..=1. Comments are skipped; selectors are not checked, since the
/// built-in rule reads the token wherever the cascade puts it. Only a plain
/// number counts: GTK's `alpha()` rejects anything else, percentages
/// included, so those are treated as unset here too.
fn opacity_in(css: &str, name: &str) -> Option<f32> {
    let mut found = None;
    let mut rest = css;
    while let Some(index) = rest.find(['/', '-']) {
        let tail = &rest[index..];
        if let Some(comment) = tail.strip_prefix("/*") {
            rest = comment.find("*/").map_or("", |end| &comment[end + 2..]);
            continue;
        }
        let Some(after) = tail.strip_prefix(name) else {
            rest = &tail[1..];
            continue;
        };
        // `--skald-sidebar-opacity-foo` is a different property.
        let Some(value) = after.trim_start().strip_prefix(':') else {
            rest = after;
            continue;
        };
        let end = value.find([';', '}', '\n']).unwrap_or(value.len());
        let parsed = value[..end].trim().parse::<f32>().ok();
        // GTK lets this last declaration win even when alpha() rejects it
        // (a percentage or a typo unsets the background), so it reads as
        // opaque rather than falling back to an earlier valid value, which
        // would clear the window behind a sidebar with no background.
        found = Some(parsed.filter(|o| o.is_finite()).map_or(1.0, |o| o.clamp(0.0, 1.0)));
        rest = &value[end..];
    }
    found
}

struct Palette {
    colors: HashMap<String, String>,
    light: bool,
}

impl Palette {
    /// Mirrors the fallback cascade used by omarchy-theme-color.
    fn resolve(source: &str, light_mode_file: bool) -> Self {
        let mut colors = parse(source);

        for (canonical, legacy) in [
            ("background", "bg"),
            ("dark_background", "dark_bg"),
            ("darker_background", "darker_bg"),
            ("lighter_background", "lighter_bg"),
            ("foreground", "fg"),
            ("dark_foreground", "dark_fg"),
            ("light_foreground", "light_fg"),
            ("bright_foreground", "bright_fg"),
        ] {
            alias(&mut colors, canonical, legacy);
        }

        alias(&mut colors, "background", "color0");
        alias(&mut colors, "foreground", "color7");
        mirror(&mut colors, "color0", "background");
        mirror(&mut colors, "color7", "foreground");

        for (name, ansi) in [
            ("red", "color1"),
            ("green", "color2"),
            ("yellow", "color3"),
            ("blue", "color4"),
            ("magenta", "color5"),
            ("cyan", "color6"),
            ("bright_red", "color9"),
            ("bright_green", "color10"),
            ("bright_yellow", "color11"),
            ("bright_blue", "color12"),
            ("bright_magenta", "color13"),
            ("bright_cyan", "color14"),
        ] {
            alias(&mut colors, name, ansi);
        }
        alias(&mut colors, "magenta", "purple");
        alias(&mut colors, "bright_magenta", "bright_purple");
        alias_any(&mut colors, "light_foreground", &["color7", "foreground"]);
        alias_any(&mut colors, "bright_foreground", &["color15", "foreground"]);
        mirror(&mut colors, "cursor", "bright_foreground");
        alias_any(&mut colors, "lighter_background", &["color0", "background"]);
        alias_any(&mut colors, "dark_foreground", &["color8", "foreground"]);
        alias_any(&mut colors, "muted", &["color8", "dark_foreground"]);
        alias_any(
            &mut colors,
            "selection",
            &["selection_background", "color8", "color0", "background"],
        );
        alias(&mut colors, "selection_background", "selection");
        alias(&mut colors, "selection_foreground", "bright_foreground");
        alias(&mut colors, "orange", "yellow");
        derive(&mut colors, "brown", "orange", "#000000", 0.5);
        derive(
            &mut colors,
            "dark_background",
            "background",
            "#000000",
            0.25,
        );
        derive(
            &mut colors,
            "darker_background",
            "background",
            "#000000",
            0.5,
        );
        for (bright, base) in [
            ("bright_red", "red"),
            ("bright_yellow", "yellow"),
            ("bright_green", "green"),
            ("bright_cyan", "cyan"),
            ("bright_blue", "blue"),
            ("bright_magenta", "magenta"),
        ] {
            derive(&mut colors, bright, base, "#ffffff", 0.2);
        }

        let light = resolve_mode(&colors, light_mode_file);
        Self { colors, light }
    }

    fn get(&self, key: &str, fallback: &str) -> String {
        self.colors
            .get(key)
            .filter(|value| !value.is_empty())
            .cloned()
            .unwrap_or_else(|| {
                self.colors
                    .get(fallback)
                    .cloned()
                    .unwrap_or_else(|| fallback.to_string())
            })
    }

    fn to_css(&self) -> String {
        let c = |key: &str, fallback: &str| self.get(key, fallback);
        let bg = c("background", "#1d1d20");
        let fg = c("foreground", "#d0cfcc");
        let fg = if self.light { ink(&fg, &bg) } else { fg };
        let accent = c("accent", "blue");
        let chrome = c("dark_background", "background");
        let backdrop = c("darker_background", "background");
        let raised = c("lighter_background", "background");
        let opacity = border_opacity(&fg, &[&bg, &chrome]);
        let border = format!("color-mix(in srgb, {fg} {opacity}%, transparent)");

        let mut css =
            String::from("/* Generated by Skald from the active Omarchy theme. */\n:root {\n");
        let mut property = |name: &str, value: &str| {
            css.push_str(&format!("  {name}: {value};\n"));
        };
        property("--accent-bg-color", &accent);
        property("--accent-fg-color", readable_on(&accent));
        property("--accent-color", &accent);
        property("--window-bg-color", &bg);
        property("--window-fg-color", &fg);
        property("--view-bg-color", &bg);
        property("--view-fg-color", &fg);
        property("--border-opacity", &format!("{opacity}%"));
        for prefix in ["--headerbar", "--sidebar", "--secondary-sidebar"] {
            property(&format!("{prefix}-bg-color"), &chrome);
            property(&format!("{prefix}-fg-color"), &fg);
            property(&format!("{prefix}-backdrop-color"), &backdrop);
            property(
                &format!("{prefix}-border-color"),
                if prefix == "--headerbar" {
                    &fg
                } else {
                    &border
                },
            );
        }
        for prefix in ["--card", "--popover", "--dialog"] {
            property(
                &format!("{prefix}-bg-color"),
                if prefix == "--card" { &raised } else { &chrome },
            );
            property(&format!("{prefix}-fg-color"), &fg);
        }
        for (prefix, base, bright) in [
            ("--success", "green", "bright_green"),
            ("--warning", "yellow", "bright_yellow"),
            ("--error", "red", "bright_red"),
            ("--destructive", "red", "bright_red"),
        ] {
            let base = c(base, base);
            property(&format!("{prefix}-bg-color"), &base);
            property(&format!("{prefix}-fg-color"), readable_on(&base));
            property(&format!("{prefix}-color"), &c(bright, &base));
        }
        css.push_str("}\n");
        css
    }
}

fn parse(source: &str) -> HashMap<String, String> {
    let mut colors = HashMap::new();
    for line in source.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key: String = key
            .chars()
            .filter(|ch| !matches!(ch, '"' | '\'' | ' ' | '\t'))
            .collect();
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        let value = match value.split_once(['"', '\'']) {
            Some((_, rest)) => rest
                .split(['"', '\''])
                .next()
                .unwrap_or_default()
                .to_string(),
            None => value.trim().to_string(),
        };
        if !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
            || !value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "#(),._+/% -".contains(ch))
        {
            continue;
        }
        colors.insert(key, value);
    }
    colors
}

fn alias(colors: &mut HashMap<String, String>, key: &str, from: &str) {
    alias_any(colors, key, &[from]);
}

fn alias_any(colors: &mut HashMap<String, String>, key: &str, from: &[&str]) {
    if colors.get(key).is_some_and(|value| !value.is_empty()) {
        return;
    }
    for candidate in from {
        if let Some(value) = colors
            .get(*candidate)
            .filter(|value| !value.is_empty())
            .cloned()
        {
            colors.insert(key.to_string(), value);
            return;
        }
    }
}

fn mirror(colors: &mut HashMap<String, String>, key: &str, from: &str) {
    if let Some(value) = colors.get(from).filter(|value| !value.is_empty()).cloned() {
        colors.insert(key.to_string(), value);
    }
}

fn derive(colors: &mut HashMap<String, String>, key: &str, base: &str, toward: &str, amount: f64) {
    if colors.get(key).is_some_and(|value| !value.is_empty()) {
        return;
    }
    let Some(base) = colors.get(base).cloned() else {
        return;
    };
    if let Some(value) = mix(&base, toward, amount) {
        colors.insert(key.to_string(), value);
    }
}

fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

fn mix(start: &str, end: &str, amount: f64) -> Option<String> {
    let (sr, sg, sb) = rgb(start)?;
    let (er, eg, eb) = rgb(end)?;
    let amount = amount.clamp(0.0, 1.0);
    let blend = |s: u8, e: u8| (s as f64 * (1.0 - amount) + e as f64 * amount + 0.5) as u8;
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        blend(sr, er),
        blend(sg, eg),
        blend(sb, eb)
    ))
}

fn luminance(color: &str) -> Option<f64> {
    let (r, g, b) = rgb(color)?;
    let linear = |channel: u8| {
        let c = channel as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Some(0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b))
}

fn contrast(a: &str, b: &str) -> Option<f64> {
    let (a, b) = (luminance(a)?, luminance(b)?);
    Some((a.max(b) + 0.05) / (a.min(b) + 0.05))
}

fn hsl(color: &str) -> Option<(f64, f64, f64)> {
    let (r, g, b) = rgb(color)?;
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let chroma = max - min;
    if chroma == 0.0 {
        return Some((0.0, 0.0, lightness));
    }
    let hue = if max == r {
        ((g - b) / chroma).rem_euclid(6.0)
    } else if max == g {
        (b - r) / chroma + 2.0
    } else {
        (r - g) / chroma + 4.0
    };
    Some((
        hue / 6.0,
        chroma / (1.0 - (2.0 * lightness - 1.0).abs()),
        lightness,
    ))
}

fn from_hsl(hue: f64, saturation: f64, lightness: f64) -> String {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue = hue * 6.0;
    let second = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hue as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let base = lightness - chroma / 2.0;
    let channel = |value: f64| ((value + base) * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", channel(r), channel(g), channel(b))
}

fn ink(fg: &str, bg: &str) -> String {
    const MAX_SATURATION: f64 = 0.30;
    const MIN_CONTRAST: f64 = 7.0;
    let Some((hue, saturation, mut lightness)) = hsl(fg) else {
        return fg.to_string();
    };
    if saturation <= MAX_SATURATION && contrast(fg, bg).is_none_or(|c| c >= MIN_CONTRAST) {
        return fg.to_string();
    }
    let saturation = saturation.min(MAX_SATURATION);
    loop {
        let candidate = from_hsl(hue, saturation, lightness);
        if lightness <= 0.0 || contrast(&candidate, bg).is_none_or(|c| c >= MIN_CONTRAST) {
            return candidate;
        }
        lightness = (lightness - 0.005).max(0.0);
    }
}

fn border_opacity(fg: &str, surfaces: &[&str]) -> u32 {
    let line_contrast =
        |surface: &str, opacity: u32| contrast(&mix(surface, fg, opacity as f64 / 100.0)?, surface);
    (15..=40)
        .find(|&opacity| {
            surfaces
                .iter()
                .all(|surface| line_contrast(surface, opacity).is_none_or(|c| c >= 1.4))
        })
        .unwrap_or(40)
}

fn readable_on(color: &str) -> &'static str {
    match rgb(color) {
        Some((r, g, b)) if 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64 > 140.0 => {
            "#000000"
        }
        _ => "#ffffff",
    }
}

fn resolve_mode(colors: &HashMap<String, String>, light_mode_file: bool) -> bool {
    for key in ["mode", "theme_type"] {
        if let Some(mode) = colors.get(key).filter(|mode| !mode.is_empty()) {
            return mode.eq_ignore_ascii_case("light");
        }
    }
    if light_mode_file {
        return true;
    }
    colors
        .get("background")
        .and_then(|bg| rgb(bg))
        .is_some_and(|(r, g, b)| r as u32 + g as u32 + b as u32 > 382)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_palette_reaches_adwaita_variables() {
        let palette = Palette::resolve(
            "mode = \"dark\"\naccent = \"#a6e3a1\"\nbackground = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\ngreen = \"#a6e3a1\"\nred = \"#f38ba8\"\nyellow = \"#f9e2af\"\n",
            false,
        );
        let css = palette.to_css();
        assert!(css.contains("--window-bg-color: #1e1e2e;"));
        assert!(css.contains("--accent-bg-color: #a6e3a1;"));
        assert!(!palette.light);
    }

    #[test]
    fn legacy_ansi_palette_and_light_mode_are_supported() {
        let palette = Palette::resolve(
            "color0 = \"#fafafa\"\ncolor4 = \"#3366ff\"\ncolor7 = \"#202020\"\n",
            false,
        );
        let css = palette.to_css();
        assert!(palette.light);
        assert!(css.contains("--window-bg-color: #fafafa;"));
        assert!(css.contains("--accent-bg-color: #3366ff;"));
    }

    /// A scratch theme directory, removed again when dropped.
    struct ThemeDir(PathBuf);

    impl ThemeDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("skald-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("colors.toml"),
                "accent = \"#a6e3a1\"\nbackground = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\n",
            )
            .unwrap();
            Self(dir)
        }
    }

    impl Drop for ThemeDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A theme's skald.css only has to name what it changes: the palette
    /// stays underneath, and the file's own declarations come last so they
    /// win the cascade.
    #[test]
    fn a_themes_own_css_is_layered_over_the_palette() {
        let dir = ThemeDir::new("layered");
        fs::write(
            dir.0.join("skald.css"),
            ":root { --skald-sidebar-opacity: 0.8; }\n",
        )
        .unwrap();
        let theme = load_in(&dir.0).unwrap();
        let palette = theme.css.find("--accent-bg-color: #a6e3a1;").unwrap();
        let own = theme.css.find("--skald-sidebar-opacity: 0.8;").unwrap();
        assert!(palette < own);
        assert!(!theme.light);
        assert_eq!(opacity_in(&theme.css, SIDEBAR_OPACITY), Some(0.8));
    }

    /// Without a skald.css the derived palette is all there is, and the
    /// sidebar keeps the built-in default.
    #[test]
    fn a_theme_without_its_own_css_leaves_the_sidebar_opaque() {
        let dir = ThemeDir::new("derived");
        let theme = load_in(&dir.0).unwrap();
        assert!(theme.css.contains("--window-bg-color: #1e1e2e;"));
        assert_eq!(opacity_in(&theme.css, SIDEBAR_OPACITY), None);
    }

    #[test]
    fn sidebar_opacity_is_read_from_any_rule_and_the_last_one_wins() {
        let css = "/* --skald-sidebar-opacity: 0.1; */\n\
                   :root {\n  --skald-sidebar-opacity: 0.8;\n}\n\
                   :root { --skald-sidebar-opacity:0.6 }\n";
        assert_eq!(opacity_in(css, SIDEBAR_OPACITY), Some(0.6));
        // Out of range clamps and look-alike names are ignored. A last value
        // GTK's alpha() rejects (a percentage, garbage) still wins the
        // cascade, so it reads as opaque rather than an earlier value.
        assert_eq!(
            opacity_in(":root { --skald-sidebar-opacity: 2; }", SIDEBAR_OPACITY),
            Some(1.0)
        );
        assert_eq!(
            opacity_in(":root { --skald-sidebar-opacity: 75%; }", SIDEBAR_OPACITY),
            Some(1.0)
        );
        assert_eq!(
            opacity_in(":root { --skald-sidebar-opacity: lots; }", SIDEBAR_OPACITY),
            Some(1.0)
        );
        assert_eq!(
            opacity_in(":root { --skald-sidebar-opacity-x: 0.5; }", SIDEBAR_OPACITY),
            None
        );
        assert_eq!(
            opacity_in("/* --skald-sidebar-opacity: 0.5", SIDEBAR_OPACITY),
            None
        );
    }
}
