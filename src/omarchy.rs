//! Follow the active Omarchy desktop palette and reload it live.

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

/// Owns the CSS provider and file monitor for the lifetime of the app.
pub struct OmarchyTheme {
    _inner: Rc<Inner>,
}

struct Inner {
    provider: gtk::CssProvider,
    monitor: RefCell<Option<gio::FileMonitor>>,
}

impl OmarchyTheme {
    /// Install above the system-accent fallback so the desktop palette wins.
    pub fn install(display: &gtk::gdk::Display) -> Self {
        let inner = Rc::new(Inner {
            provider: gtk::CssProvider::new(),
            monitor: RefCell::new(None),
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
        Self { _inner: inner }
    }
}

impl Inner {
    fn reload(&self) {
        let theme = load(&glib::home_dir());
        self.provider
            .load_from_string(theme.as_ref().map_or("", |theme| theme.css.as_str()));
        adw::StyleManager::default().set_color_scheme(match theme {
            Some(theme) if theme.light => adw::ColorScheme::ForceLight,
            Some(_) => adw::ColorScheme::ForceDark,
            None => adw::ColorScheme::Default,
        });
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
    let dir = theme_dir(home);
    let colors = fs::read_to_string(dir.join("colors.toml")).ok()?;
    let palette = Palette::resolve(&colors, dir.join("light.mode").exists());
    // A theme may provide app-specific CSS, just as it can for Rustle.
    let css = fs::read_to_string(dir.join("skald.css")).unwrap_or_else(|_| palette.to_css());
    Some(Theme {
        css,
        light: palette.light,
    })
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
}
