// Theme watching follows Omacut (MIT, David Heinemeier Hansson); the palette
// is the one Skald's GTK version and Bubo derive for libadwaita.
#include "theme.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QRegularExpression>
#include <QStandardPaths>
#include <QStyleHints>
#include <cmath>

namespace {

using Colors = QHash<QString, QString>;

QString readFile(const QString &path) {
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? QString::fromUtf8(file.readAll()) : QString();
}

Colors parseToml(const QString &source) {
    static const QRegularExpression keyCharacters("^[A-Za-z0-9_-]+$");
    static const QRegularExpression valueCharacters("^[A-Za-z0-9#(),._+/% -]*$");
    Colors colors;
    for (const auto &line : source.split('\n')) {
        const int equals = line.indexOf('=');
        if (equals < 0)
            continue;
        QString key = line.left(equals);
        key.remove(QRegularExpression("[\"'\\s]"));
        if (key.isEmpty() || key.startsWith('#') || !keyCharacters.match(key).hasMatch())
            continue;
        QString value = line.mid(equals + 1);
        const int quote = value.indexOf(QRegularExpression("[\"']"));
        if (quote >= 0) {
            value = value.mid(quote + 1);
            value = value.left(value.indexOf(QRegularExpression("[\"']")));
        } else {
            value = value.trimmed();
        }
        if (valueCharacters.match(value).hasMatch())
            colors.insert(key, value);
    }
    return colors;
}

bool has(const Colors &colors, const QString &key) { return !colors.value(key).isEmpty(); }

void aliasAny(Colors &colors, const QString &key, const QStringList &from) {
    if (has(colors, key))
        return;
    for (const auto &candidate : from)
        if (has(colors, candidate)) {
            colors.insert(key, colors.value(candidate));
            return;
        }
}
void alias(Colors &colors, const QString &key, const QString &from) { aliasAny(colors, key, {from}); }
void mirror(Colors &colors, const QString &key, const QString &from) {
    if (has(colors, from))
        colors.insert(key, colors.value(from));
}

QColor hex(const QString &value) {
    static const QRegularExpression sixDigits("^#[0-9a-fA-F]{6}$");
    return sixDigits.match(value).hasMatch() ? QColor(value) : QColor();
}

QColor mix(const QColor &start, const QColor &end, double amount) {
    amount = std::clamp(amount, 0.0, 1.0);
    auto blend = [amount](int s, int e) { return int(s * (1 - amount) + e * amount + 0.5); };
    return QColor(blend(start.red(), end.red()), blend(start.green(), end.green()), blend(start.blue(), end.blue()));
}

void derive(Colors &colors, const QString &key, const QString &base, const QString &toward, double amount) {
    if (has(colors, key))
        return;
    const QColor from = hex(colors.value(base));
    if (from.isValid())
        colors.insert(key, mix(from, QColor(toward), amount).name());
}

double luminance(const QColor &c) {
    auto linear = [](double v) { return v <= .04045 ? v / 12.92 : std::pow((v + .055) / 1.055, 2.4); };
    return .2126 * linear(c.redF()) + .7152 * linear(c.greenF()) + .0722 * linear(c.blueF());
}

double contrast(const QColor &a, const QColor &b) {
    const double la = luminance(a), lb = luminance(b);
    return (std::max(la, lb) + .05) / (std::min(la, lb) + .05);
}

// A light theme's foreground made into ink: desaturated and dark enough to
// read on the background, as the GTK palette does.
QColor ink(const QColor &fg, const QColor &bg) {
    constexpr double maxSaturation = .30, minContrast = 7.0;
    if (!fg.isValid() || !bg.isValid())
        return fg;
    if (fg.hslSaturationF() <= maxSaturation && contrast(fg, bg) >= minContrast)
        return fg;
    const double hue = std::max<float>(0, fg.hslHueF()), saturation = std::min<float>(fg.hslSaturationF(), maxSaturation);
    double lightness = fg.lightnessF();
    for (;;) {
        const QColor candidate = QColor::fromHslF(hue, saturation, lightness);
        if (lightness <= 0 || contrast(candidate, bg) >= minContrast)
            return candidate;
        lightness = std::max(0.0, lightness - .005);
    }
}

// The faintest line, in percent of the foreground, that still shows on every surface.
int borderOpacity(const QColor &fg, const QList<QColor> &surfaces) {
    for (int opacity = 15; opacity <= 40; ++opacity) {
        bool visible = true;
        for (const auto &surface : surfaces)
            if (surface.isValid() && fg.isValid() && contrast(mix(surface, fg, opacity / 100.0), surface) < 1.4)
                visible = false;
        if (visible)
            return opacity;
    }
    return 40;
}

QString readableOn(const QString &color) {
    const QColor c(color);
    return c.isValid() && .299 * c.red() + .587 * c.green() + .114 * c.blue() > 140 ? "#000000" : "#ffffff";
}

QString withAlpha(const QString &color, int percent) {
    QColor c(color);
    c.setAlphaF(percent / 100.0);
    return c.name(QColor::HexArgb);
}

// libadwaita's own colors, for a desktop without Omarchy.
Colors adwaita(bool dark) {
    if (dark)
        return {{"--window-bg-color", "#222226"}, {"--window-fg-color", "#ffffff"},
                {"--headerbar-bg-color", "#2e2e32"}, {"--headerbar-shade-color", "#5c000006"},
                {"--sidebar-bg-color", "#2e2e32"}, {"--sidebar-backdrop-color", "#28282c"},
                {"--card-bg-color", "#14ffffff"}, {"--popover-bg-color", "#36363a"},
                {"--border-color", "#26ffffff"}, {"--accent-bg-color", "#3584e4"},
                {"--accent-fg-color", "#ffffff"}, {"--error-color", "#ff938c"}};
    return {{"--window-bg-color", "#fafafb"}, {"--window-fg-color", "#cc000006"},
            {"--headerbar-bg-color", "#ffffff"}, {"--headerbar-shade-color", "#1f000006"},
            {"--sidebar-bg-color", "#ebebed"}, {"--sidebar-backdrop-color", "#f2f2f4"},
            {"--card-bg-color", "#ffffff"}, {"--popover-bg-color", "#ffffff"},
            {"--border-color", "#26000006"}, {"--accent-bg-color", "#3584e4"},
            {"--accent-fg-color", "#ffffff"}, {"--error-color", "#c30000"}};
}

bool systemIsDark() {
    return qGuiApp && qGuiApp->styleHints()->colorScheme() == Qt::ColorScheme::Dark;
}

} // namespace

// Mirrors the fallback cascade used by omarchy-theme-color.
QHash<QString, QString> Theme::palette(const QString &colorsToml, bool lightModeFile, bool *lightOut) {
    Colors colors = parseToml(colorsToml);

    const QList<QPair<QString, QString>> legacy{
        {"background", "bg"}, {"dark_background", "dark_bg"}, {"darker_background", "darker_bg"},
        {"lighter_background", "lighter_bg"}, {"foreground", "fg"}, {"dark_foreground", "dark_fg"},
        {"light_foreground", "light_fg"}, {"bright_foreground", "bright_fg"}};
    for (const auto &[canonical, old] : legacy)
        alias(colors, canonical, old);
    alias(colors, "background", "color0");
    alias(colors, "foreground", "color7");
    mirror(colors, "color0", "background");
    mirror(colors, "color7", "foreground");
    const QList<QPair<QString, QString>> ansi{
        {"red", "color1"}, {"green", "color2"}, {"yellow", "color3"}, {"blue", "color4"},
        {"magenta", "color5"}, {"cyan", "color6"}, {"bright_red", "color9"}, {"bright_green", "color10"},
        {"bright_yellow", "color11"}, {"bright_blue", "color12"}, {"bright_magenta", "color13"},
        {"bright_cyan", "color14"}};
    for (const auto &[name, number] : ansi)
        alias(colors, name, number);
    alias(colors, "magenta", "purple");
    aliasAny(colors, "light_foreground", {"color7", "foreground"});
    aliasAny(colors, "bright_foreground", {"color15", "foreground"});
    aliasAny(colors, "lighter_background", {"color0", "background"});
    aliasAny(colors, "dark_foreground", {"color8", "foreground"});
    derive(colors, "dark_background", "background", "#000000", .25);
    derive(colors, "darker_background", "background", "#000000", .5);
    derive(colors, "bright_red", "red", "#ffffff", .2);

    bool light = false;
    if (has(colors, "mode") || has(colors, "theme_type")) {
        light = (has(colors, "mode") ? colors.value("mode") : colors.value("theme_type")).compare("light", Qt::CaseInsensitive) == 0;
    } else if (lightModeFile) {
        light = true;
    } else {
        const QColor bg = hex(colors.value("background"));
        light = bg.isValid() && bg.red() + bg.green() + bg.blue() > 382;
    }
    if (lightOut)
        *lightOut = light;

    auto get = [&colors](const QString &key, const QString &fallback) {
        return has(colors, key) ? colors.value(key) : colors.value(fallback, fallback);
    };
    const QString bg = get("background", "#1d1d20");
    QString fg = get("foreground", "#d0cfcc");
    if (light && hex(fg).isValid() && hex(bg).isValid())
        fg = ink(hex(fg), hex(bg)).name();
    const QString accent = get("accent", "#3584e4");
    const QString chrome = get("dark_background", bg);
    const QString backdrop = get("darker_background", bg);
    const QString raised = get("lighter_background", bg);
    const QString red = get("bright_red", get("red", light ? "#c30000" : "#ff938c"));
    const int opacity = borderOpacity(QColor(fg), {QColor(bg), QColor(chrome)});

    return {{"--window-bg-color", bg}, {"--window-fg-color", fg},
            {"--headerbar-bg-color", chrome}, {"--headerbar-shade-color", light ? "#1f000006" : "#5c000006"},
            {"--sidebar-bg-color", chrome}, {"--sidebar-backdrop-color", backdrop},
            {"--card-bg-color", raised}, {"--popover-bg-color", chrome},
            {"--border-color", withAlpha(fg, opacity)}, {"--accent-bg-color", accent},
            {"--accent-fg-color", readableOn(accent)}, {"--error-color", red}};
}

QHash<QString, QString> Theme::customProperties(const QString &css) {
    QString text = css;
    text.remove(QRegularExpression(R"(/\*.*?(\*/|$))", QRegularExpression::DotMatchesEverythingOption));
    static const QRegularExpression declaration(R"((?<![\w-])(--[\w-]+)\s*:\s*([^;}\n]*))");
    QHash<QString, QString> found;
    for (auto it = declaration.globalMatch(text); it.hasNext();) {
        const auto match = it.next();
        found.insert(match.captured(1), match.captured(2).trimmed());
    }
    return found;
}

double Theme::sidebarOpacityIn(const QString &css) {
    const auto properties = customProperties(css);
    if (!properties.contains("--skald-sidebar-opacity"))
        return 1;
    bool ok = false;
    const double value = properties.value("--skald-sidebar-opacity").toDouble(&ok);
    return ok && std::isfinite(value) ? std::clamp(value, 0.0, 1.0) : 1;
}

Theme::Theme(const QString &directory, QObject *parent)
    : QObject(parent),
      m_directory(directory.isEmpty() ? QDir::homePath() + "/.local/state/omarchy/current" : directory),
      m_settings(directory.isEmpty()
                     ? QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation) + "/skald/skald.ini"
                     : directory + "/skald.ini",
                 QSettings::IniFormat) {
    m_debounce.setSingleShot(true);
    m_debounce.setInterval(80);
    connect(&m_watcher, &QFileSystemWatcher::directoryChanged, this, [this] { m_debounce.start(); });
    connect(&m_watcher, &QFileSystemWatcher::fileChanged, this, [this] { m_debounce.start(); });
    connect(&m_debounce, &QTimer::timeout, this, &Theme::reload);
    if (qGuiApp)
        connect(qGuiApp->styleHints(), &QStyleHints::colorSchemeChanged, this, &Theme::reload);
    reload();
    loadSidebarSetting();
}

// The first launch with the setting adopts what the theme's skald.css used to
// decide, so a see-through sidebar stays see-through; after that the theme
// has no say.
void Theme::loadSidebarSetting() {
    if (!m_settings.contains("transparentSidebar")) {
        if (m_themeSidebarOpacity < 1) {
            m_transparentSidebar = true;
            m_sidebarOpacitySetting = m_themeSidebarOpacity;
        }
        m_settings.setValue("transparentSidebar", m_transparentSidebar);
        m_settings.setValue("sidebarOpacity", m_sidebarOpacitySetting);
        return;
    }
    m_transparentSidebar = m_settings.value("transparentSidebar").toBool();
    bool ok = false;
    const double opacity = m_settings.value("sidebarOpacity", 0.8).toDouble(&ok);
    m_sidebarOpacitySetting = ok && std::isfinite(opacity) ? std::clamp(opacity, 0.0, 1.0) : 0.8;
}

void Theme::setTransparentSidebar(bool transparent) {
    if (transparent == m_transparentSidebar)
        return;
    m_transparentSidebar = transparent;
    m_settings.setValue("transparentSidebar", transparent);
    emit changed();
}

void Theme::setSidebarOpacitySetting(double opacity) {
    // Two places, as the slider shows it.
    opacity = std::round(std::clamp(opacity, 0.0, 1.0) * 100) / 100;
    if (opacity == m_sidebarOpacitySetting)
        return;
    m_sidebarOpacitySetting = opacity;
    m_settings.setValue("sidebarOpacity", opacity);
    emit changed();
}

QColor Theme::color(const QString &name) const {
    const QColor value = QColor::fromString(m_variables.value(name));
    return value.isValid() ? value : QColor::fromString(adwaita(m_dark).value(name));
}

void Theme::reload() {
    watch();
    const QString dir = m_directory + "/theme";
    const QString toml = readFile(dir + "/colors.toml");
    bool light = !systemIsDark();
    QHash<QString, QString> variables = toml.isEmpty() ? adwaita(!light)
                                                       : palette(toml, QFileInfo::exists(dir + "/light.mode"), &light);

    // The theme's own skald.css is layered over the palette, so a partial
    // file only changes what it names. Values Qt can't read as a color are
    // left to the palette.
    const QString css = toml.isEmpty() ? QString() : readFile(dir + "/skald.css");
    const auto own = customProperties(css);
    for (auto it = own.begin(); it != own.end(); ++it)
        if (variables.contains(it.key()) && QColor::fromString(it.value()).isValid())
            variables.insert(it.key(), it.value());
    if (own.contains("--accent-bg-color") && !own.contains("--accent-fg-color"))
        variables.insert("--accent-fg-color", readableOn(variables.value("--accent-bg-color")));

    m_themeSidebarOpacity = sidebarOpacityIn(css);
    if (variables == m_variables && light == !m_dark)
        return;
    m_variables = variables;
    m_dark = !light;
    emit changed();
}

void Theme::watch() {
    const auto paths = m_watcher.files() + m_watcher.directories();
    if (!paths.isEmpty())
        m_watcher.removePaths(paths);

    // A theme switch replaces symlinks and files, so watch the parents too and
    // re-arm after every change. Watching the nearest existing ancestor means
    // installing Omarchy later works without restarting the app.
    QString ancestor = m_directory;
    while (!QFileInfo::exists(ancestor) && ancestor != "/")
        ancestor = QFileInfo(ancestor).absolutePath();
    QStringList candidates{ancestor, QFileInfo(ancestor).absolutePath(), m_directory,
                           m_directory + "/theme", m_directory + "/theme/colors.toml",
                           m_directory + "/theme/skald.css"};
    candidates.removeDuplicates();
    for (const auto &path : candidates)
        if (QFileInfo::exists(path))
            m_watcher.addPath(path);
}
