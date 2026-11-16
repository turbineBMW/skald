#pragma once

#include <QColor>
#include <QFileSystemWatcher>
#include <QHash>
#include <QObject>
#include <QTimer>

// The Omarchy palette, resolved the way Skald's GTK siblings (Bubo, Rustle)
// resolve it for libadwaita, so the window sits beside them as one of theirs.
//
// colors.toml gives the colors and light or dark mode. A theme's skald.css
// (or ~/.config/omarchy/themed/skald.css.tpl) is layered over that: it can
// restate any of the libadwaita color variables below, and set
// --skald-sidebar-opacity, a number from 0 to 1 that fades the chapter
// sidebar's background, never its text.
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
    Q_PROPERTY(double sidebarOpacity READ sidebarOpacity NOTIFY changed)

public:
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
    double sidebarOpacity() const { return m_sidebarOpacity; }

    // The libadwaita variables a colors.toml resolves to, and whether it is light.
    static QHash<QString, QString> palette(const QString &colorsToml, bool lightModeFile, bool *light = nullptr);
    // Every custom property `css` sets, the last value winning. Comments are skipped.
    static QHash<QString, QString> customProperties(const QString &css);
    // --skald-sidebar-opacity as `css` sets it: 1 when unset, and also when
    // the last value isn't a plain number, as GTK would read it.
    static double sidebarOpacityIn(const QString &css);

signals:
    void changed();

private:
    void reload();
    void watch();
    QColor color(const QString &name) const;

    QString m_directory;
    QHash<QString, QString> m_variables;
    bool m_dark = true;
    double m_sidebarOpacity = 1;
    QFileSystemWatcher m_watcher;
    QTimer m_debounce;
};
