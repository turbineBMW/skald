// skald — a desktop player for your Audible library, built on Audible's own web player.

#include <QDir>
#include <QGuiApplication>
#include <QIcon>
#include <QNetworkAccessManager>
#include <QNetworkDiskCache>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlNetworkAccessManagerFactory>
#include <QQuickStyle>
#include <QQuickWebEngineProfile>
#include <QRegularExpression>
#include <QStandardPaths>
#include <QUrl>
#include <QWebEngineCookieStore>
#include <QtWebEngineQuick>

#include "backend.h"
#include "mpris.h"
#include "theme.h"
#include "translator.h"

// Covers come from Amazon's image host; keeping them on disk means the
// library draws at once on the next start instead of fetching 200 images.
class CoverCache : public QQmlNetworkAccessManagerFactory {
public:
    QNetworkAccessManager *create(QObject *parent) override {
        auto *manager = new QNetworkAccessManager(parent);
        auto *cache = new QNetworkDiskCache(manager);
        cache->setCacheDirectory(QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation)
                                 + "/skald/cover-cache");
        cache->setMaximumCacheSize(200 * 1024 * 1024);
        manager->setCache(cache);
        return manager;
    }
};

int main(int argc, char *argv[]) {
    // Has to run before the application object exists.
    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    app.setApplicationName("skald");
    app.setApplicationVersion("0.2.0");

    // Matches the window to skald.desktop, so the compositor (Wayland app_id)
    // and the launcher pick up the installed icon.
    app.setDesktopFileName("skald");
    app.setWindowIcon(QIcon::fromTheme("skald"));

    QQuickStyle::setStyle("Material");

    // Installed before anything is translated, and every qsTr() binding is
    // re-evaluated when the Omarchy language changes.
    Translator translator;
    app.installTranslator(&translator);

    Theme theme;
    Backend backend;
    Mpris mpris(&backend);

    // One browser profile for both pages, kept on disk so the Audible
    // sign-in survives a restart. It presents as the Chromium it is, without
    // the Qt token some sites treat as an unknown browser.
    QQuickWebEngineProfile profile("skald");
    profile.setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
    profile.setHttpUserAgent(profile.httpUserAgent().remove(QRegularExpression(" QtWebEngine/\\S+")));
    QObject::connect(&backend, &Backend::clearSession, &profile, [&profile] {
        profile.cookieStore()->deleteAllCookies();
        profile.clearHttpCache();
    });

    CoverCache covers;
    QQmlApplicationEngine engine;
    engine.setNetworkAccessManagerFactory(&covers);
    QObject::connect(&translator, &Translator::changed, &engine, &QQmlEngine::retranslate);
    engine.rootContext()->setContextProperty("translator", &translator);
    engine.rootContext()->setContextProperty("theme", &theme);
    engine.rootContext()->setContextProperty("backend", &backend);
    engine.rootContext()->setContextProperty("webProfile", &profile);
    engine.load(QUrl("qrc:/Main.qml"));
    if (engine.rootObjects().isEmpty())
        return 1;

    return app.exec();
}
