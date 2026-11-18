#pragma once

#include <QColor>
#include <QFileSystemWatcher>
#include <QHash>
#include <QObject>
#include <QSettings>
#include <QTimer>

// The Omarchy palette, resolved the way Skald's GTK siblings (Bubo, Rustle)
// resolve it for libadwaita, so the window sits beside them as one of theirs.
//
// colors.toml gives the colors and light or dark mode. A theme's skald.css
// (or ~/.config/omarchy/themed/skald.css.tpl) is layered over that: it can
// restate any of the libadwaita color variables below.
//
// The see-through chapter sidebar is Skald's own setting (Preferences →
// Transparent sidebar, kept in skald.ini), not the theme's: sidebarOpacity
// is the setting's opacity while it is on and 1 otherwise. Themes used to set
// --skald-sidebar-opacity; the first launch with the setting carries a
// theme's value over, once.
//
// Without Omarchy, the window follows the system's light or dark preference
// in libadwaita's own default colors.
class Theme : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool dark READ dark NOTIFY changed)
    Q_PROPERTY(QColor window READ window NOTIFY changed)
    Q_PROPERTY(QColor foreground READ foreground NOTIFY changed)
    Q_PROPERTY(QColor headerbar READ headerbar NOTIFY changed)
    Q_PROPERTY(QColor headerbarShade READ headerbarShade NOTIFY changed)
    Q_PROPERTY(QColor sidebar READ sidebar NOTIFY changed)
    Q_PROPERTY(QColor sidebarBackdrop READ sidebarBackdrop NOTIFY changed)
    Q_PROPERTY(QColor card READ card NOTIFY changed)
    Q_PROPERTY(QColor popover READ popover NOTIFY changed)
    Q_PROPERTY(QColor border READ border NOTIFY changed)
    Q_PROPERTY(QColor accent READ accent NOTIFY changed)
    Q_PROPERTY(QColor accentForeground READ accentForeground NOTIFY changed)
    Q_PROPERTY(QColor error READ error NOTIFY changed)
    // The chapter sidebar's background opacity now in force.
    Q_PROPERTY(double sidebarOpacity READ sidebarOpacity NOTIFY changed)
    Q_PROPERTY(bool transparentSidebar READ transparentSidebar WRITE setTransparentSidebar NOTIFY changed)
    // What the Sidebar opacity slider is set to, from 0 to 1.
    Q_PROPERTY(double sidebarOpacitySetting READ sidebarOpacitySetting WRITE setSidebarOpacitySetting NOTIFY changed)

public:
    // Tests pass a directory, which then holds the Omarchy state and skald.ini.
    explicit Theme(const QString &currentDirectory = {}, QObject *parent = nullptr);

    bool dark() const { return m_dark; }
    QColor window() const { return color("--window-bg-color"); }
    QColor foreground() const { return color("--window-fg-color"); }
    QColor headerbar() const { return color("--headerbar-bg-color"); }
    QColor headerbarShade() const { return color("--headerbar-shade-color"); }
    QColor sidebar() const { return color("--sidebar-bg-color"); }
    QColor sidebarBackdrop() const { return color("--sidebar-backdrop-color"); }
    QColor card() const { return color("--card-bg-color"); }
    QColor popover() const { return color("--popover-bg-color"); }
    QColor border() const { return color("--border-color"); }
    QColor accent() const { return color("--accent-bg-color"); }
    QColor accentForeground() const { return color("--accent-fg-color"); }
    QColor error() const { return color("--error-color"); }
    double sidebarOpacity() const { return m_transparentSidebar ? m_sidebarOpacitySetting : 1; }
    bool transparentSidebar() const { return m_transparentSidebar; }
    void setTransparentSidebar(bool transparent);
    double sidebarOpacitySetting() const { return m_sidebarOpacitySetting; }
    void setSidebarOpacitySetting(double opacity);

    // The libadwaita variables a colors.toml resolves to, and whether it is light.
    static QHash<QString, QString> palette(const QString &colorsToml, bool lightModeFile, bool *light = nullptr);
    // Every custom property `css` sets, the last value winning. Comments are skipped.
    static QHash<QString, QString> customProperties(const QString &css);
    // --skald-sidebar-opacity as `css` sets it: 1 when unset, and also when
    // the last value isn't a plain number, as GTK would read it. Only read
    // once now, to carry a theme's old value into the setting.
    static double sidebarOpacityIn(const QString &css);

signals:
    void changed();

private:
    void reload();
    void watch();
    void loadSidebarSetting();
    QColor color(const QString &name) const;

    QString m_directory;
    QHash<QString, QString> m_variables;
    bool m_dark = true;
    // What the active theme's skald.css says, for the one-time carry-over.
    double m_themeSidebarOpacity = 1;
    QSettings m_settings;
    bool m_transparentSidebar = false;
    double m_sidebarOpacitySetting = 0.8;
    QFileSystemWatcher m_watcher;
    QTimer m_debounce;
};
