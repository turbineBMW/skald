#include <QDir>
#include <QFile>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QtTest>

#include "backend.h"
#include "theme.h"
#include "translator.h"

// No "//" inside these raw strings: moc reads it as a comment and loses the class.
static const char *TwoBooks = R"({"status":"done","loaded":2,"pages":1,"error":"","books":[
  {"asin":"A1","title":"The Long Winter","authors":["Ada Frost"],"narrators":["Sam Reed"],"series":"Seasons",
   "cover":"covers/a1.jpg","percent":40,"remaining":"6h 2m left","finished":false},
  {"asin":"A2","title":"Harbor Lights","authors":["Mina Vale","Ada Frost"],"narrators":[],"series":"",
   "cover":"","percent":0,"remaining":"","finished":true},
  {"asin":"","title":"No asin is dropped"}]})";

class SkaldTests : public QObject {
    Q_OBJECT

private slots:
    void clockFormatsHoursOnlyWhenNeeded()
    {
        QCOMPARE(Backend::clock(0), QString("0:00"));
        QCOMPARE(Backend::clock(123.9), QString("2:03"));
        QCOMPARE(Backend::clock(3723), QString("1:02:03"));
        QCOMPARE(Backend::clock(-5), QString("0:00"));
    }

    void signInPagesAreRecognized()
    {
        QVERIFY(Backend::isSignIn(QUrl("https://www.amazon.com/ap/signin?openid=1")));
        QVERIFY(Backend::isSignIn(QUrl("https://www.audible.com/ap/return")));
        QVERIFY(Backend::isSignIn(QUrl("https://www.amazon.co.uk/ap/cvf/request")));
        QVERIFY(!Backend::isSignIn(QUrl("https://www.audible.com/library/titles")));
        QVERIFY(!Backend::isSignIn(QUrl("https://www.audible.com/webplayer?asin=A1")));
    }

    void sessionArrivingAtTheLibraryStartsARefresh()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        QSignalSpy run(&backend, &Backend::runInSession);
        QCOMPARE(backend.state(), QString("starting"));

        backend.sessionNavigated(QUrl("https://www.amazon.com/ap/signin"), false);
        QCOMPARE(backend.state(), QString("signin"));

        backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), false);
        QCOMPARE(backend.state(), QString("signin")); // not loaded yet
        backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), true);
        QCOMPARE(backend.state(), QString("ready"));
        QVERIFY(backend.refreshing());
        QCOMPARE(run.count(), 1);
        QVERIFY(run.first().first().toString().contains("__skaldLibrary.start()"));
    }

    void signedInElsewhereGoesToCheckTheLibrary()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        QSignalSpy load(&backend, &Backend::loadSession);
        backend.sessionNavigated(QUrl("https://www.amazon.com/ap/signin"), true);
        backend.sessionNavigated(QUrl("https://www.audible.com/"), true);
        QCOMPARE(load.count(), 1);
        QCOMPARE(load.first().first().toUrl(), backend.libraryUrl());
    }

    void libraryIsReadFilteredAndCached()
    {
        QTemporaryDir dir;
        {
            Backend backend(dir.path());
            backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), true);
            backend.ingestLibrary(R"({"status":"loading","loaded":50,"pages":3,"error":""})");
            QVERIFY(backend.refreshing());
            QCOMPARE(backend.refreshLoaded(), 50);

            backend.ingestLibrary(TwoBooks);
            QVERIFY(!backend.refreshing());
            QCOMPARE(backend.library()->total(), 2);
            QCOMPARE(backend.library()->asinAt(1), QString("A2"));

            backend.setFilter("frost harbor");
            QCOMPARE(backend.library()->count(), 1);
            QCOMPARE(backend.library()->asinAt(0), QString("A2"));
            QCOMPARE(backend.library()->rowOf("A1"), -1);
            backend.setFilter("SEASONS");
            QCOMPARE(backend.library()->asinAt(0), QString("A1"));
        }
        // A new start shows the last library before anything is fetched.
        Backend again(dir.path());
        QCOMPARE(again.library()->total(), 2);
        QCOMPARE(again.library()->find("A1")->remaining, QString("6h 2m left"));
        QVERIFY(again.library()->find("A2")->finished);
    }

    void libraryErrorsAreReported()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), true);
        backend.ingestLibrary(R"({"status":"error","loaded":0,"pages":0,"error":"HTTP 503"})");
        QVERIFY(!backend.refreshing());
        QVERIFY(backend.libraryError().contains("HTTP 503"));
    }

    void expiredSessionDuringRefreshAsksToSignIn()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), true);
        backend.ingestLibrary(R"({"status":"signin","loaded":0,"pages":0,"error":""})");
        QCOMPARE(backend.state(), QString("signin"));
        QVERIFY(!backend.refreshing());
    }

    void openingABookLoadsThePlayerAndRemembersIt()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), true);
        backend.ingestLibrary(TwoBooks);
        QSignalSpy load(&backend, &Backend::loadPlayer);

        backend.openBook("A1");
        QCOMPARE(load.count(), 1);
        QCOMPARE(load.first().first().toUrl(), QUrl("https://www.audible.com/webplayer?asin=A1"));
        QCOMPARE(backend.title(), QString("The Long Winter"));
        QCOMPARE(backend.author(), QString("Ada Frost"));
        QVERIFY(!backend.ready());
        QCOMPARE(backend.lastAsin(), QString("A1"));

        backend.openBook("A1"); // already open
        QCOMPARE(load.count(), 1);
    }

    void playerStateIsFollowed()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        backend.openBook("A9"); // not in the library: the page supplies the details
        QSignalSpy run(&backend, &Backend::runInPlayer);

        backend.ingestPlayer("");
        QVERIFY(!backend.ready());

        backend.ingestPlayer(R"({"path":"/webplayer","ready":true,"playing":true,"position":130.5,"duration":3600,
            "rate":1,"title":"Night Ferry","chapter":"Two","cover":"covers/c.jpg",
            "chapterPosition":30.5,"chapterDuration":200,"error":"",
            "chapters":[{"title":"One","duration":100},{"title":"Two","duration":200},{"title":"Two","duration":300}]})");
        QVERIFY(backend.ready());
        QVERIFY(backend.playing());
        QCOMPARE(backend.title(), QString("Night Ferry"));
        QCOMPARE(backend.chapters().size(), 3);
        QCOMPARE(backend.chapterIndex(), 1);
        QCOMPARE(backend.chapterDuration(), 200.0);
        QCOMPARE(run.count(), 0); // already at the chosen speed

        // A report without the list keeps the chapters; a sign-in page is not a player.
        backend.ingestPlayer(R"({"path":"/ap/signin","ready":true,"playing":true})");
        QVERIFY(!backend.ready());
        QVERIFY(!backend.playing());
        QCOMPARE(backend.chapters().size(), 3);
    }

    void repeatedChapterTitlesResolveByPosition()
    {
        const QVariantList chapters{QVariantMap{{"title", "Interlude"}, {"duration", 100.0}},
                                    QVariantMap{{"title", "One"}, {"duration", 500.0}},
                                    QVariantMap{{"title", "Interlude"}, {"duration", 100.0}}};
        QCOMPARE(Backend::chapterIndexFor(chapters, "Interlude", 2.0), 0);
        QCOMPARE(Backend::chapterIndexFor(chapters, "Interlude", 598.0), 2);
        QCOMPARE(Backend::chapterIndexFor(chapters, "One", 0.0), 1);
        QCOMPARE(Backend::chapterIndexFor(chapters, "Missing", 0.0), -1);
    }

    void speedIsBoundedKeptAndSentToThePage()
    {
        QTemporaryDir dir;
        {
            Backend backend(dir.path());
            backend.openBook("A1");
            QSignalSpy run(&backend, &Backend::runInPlayer);
            backend.stepRate(3);
            QCOMPARE(backend.rate(), 1.3);
            QVERIFY(run.last().first().toString().contains("command('rate', 1.30)"));
            backend.stepRate(100);
            QCOMPARE(backend.rate(), 3.5);
            backend.stepRate(-100);
            QCOMPARE(backend.rate(), 0.5);
            backend.stepRate(7);

            // A page still at its own speed is brought to the chosen one.
            run.clear();
            backend.ingestPlayer(R"({"path":"/webplayer","ready":true,"rate":1})");
            QCOMPARE(run.count(), 1);
            QVERIFY(run.first().first().toString().contains("command('rate', 1.20)"));
        }
        Backend again(dir.path());
        QCOMPARE(again.rate(), 1.2);
    }

    void commandsNeedABook()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        QSignalSpy run(&backend, &Backend::runInPlayer);
        backend.toggle();
        backend.skip(30);
        QCOMPARE(run.count(), 0);
        backend.openBook("A1");
        backend.skip(-30);
        backend.jumpToChapter(4);
        QCOMPARE(run.count(), 2);
        QVERIFY(run.at(0).first().toString().contains("command('skip', -30.0)"));
        QVERIFY(run.at(1).first().toString().contains("command('chapter', 4)"));
    }

    void signingOutForgetsTheLibrary()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        backend.sessionNavigated(QUrl("https://www.audible.com/library/titles"), true);
        backend.ingestLibrary(TwoBooks);
        backend.openBook("A1");
        QSignalSpy clear(&backend, &Backend::clearSession);

        backend.signOut();
        QCOMPARE(clear.count(), 1);
        QCOMPARE(backend.library()->total(), 0);
        QCOMPARE(backend.asin(), QString());
        QVERIFY(!QFile::exists(dir.path() + "/library.json"));
        QCOMPARE(Backend(dir.path()).lastAsin(), QString());
    }

    void widevineIsFoundInABrowserProfile()
    {
        QTemporaryDir dir;
        QCOMPARE(Backend::findWidevine({dir.path()}), QString());
        const QString library = dir.path() + "/4.10.1/_platform_specific/linux_x64/libwidevinecdm.so";
        QVERIFY(QDir().mkpath(QFileInfo(library).absolutePath()));
        write(library, "x");
        QCOMPARE(Backend::findWidevine({dir.path() + "/missing", dir.path()}), library);
    }

    void pageScriptsAreBundled()
    {
        QTemporaryDir dir;
        Backend backend(dir.path());
        QVERIFY(backend.script("player").contains("window.__skald ="));
        QVERIFY(backend.script("library").contains("window.__skaldLibrary ="));
        QCOMPARE(backend.script("nope"), QString());
    }

    void themeFallsBackToAdwaita()
    {
        QTemporaryDir dir;
        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QColor("#3584e4"));
        QCOMPARE(theme.sidebarOpacity(), 1.0);
    }

    void themeResolvesTheOmarchyPalette()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml",
              "mode = \"dark\"\naccent = \"#112233\"\nbackground = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\n");

        Theme theme(dir.path());
        QVERIFY(theme.dark());
        QCOMPARE(theme.accent(), QColor("#112233"));
        QCOMPARE(theme.accentForeground(), QColor("#ffffff"));
        QCOMPARE(theme.window(), QColor("#1e1e2e"));
        QCOMPARE(theme.foreground(), QColor("#cdd6f4"));
        // No dark_background: a quarter of the way to black, like omarchy-theme-color.
        QCOMPARE(theme.sidebar(), QColor("#171723"));
        QCOMPARE(theme.headerbar(), theme.sidebar());
    }

    void themeFollowsLightMode()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml", "color0 = \"#f7efe6\"\ncolor7 = \"#2a2329\"\naccent = \"#e8894a\"\n");
        write(dir.path() + "/theme/light.mode", "");

        Theme theme(dir.path());
        QVERIFY(!theme.dark());
        QCOMPARE(theme.window(), QColor("#f7efe6"));
        QCOMPARE(theme.accentForeground(), QColor("#000000"));
    }

    void themeCssIsLayeredOverThePalette()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml", "background = \"#1e1e2e\"\naccent = \"#112233\"\n");
        write(dir.path() + "/theme/skald.css",
              "/* --skald-sidebar-opacity: 0.1; */\n:root { --skald-sidebar-opacity: 0.8; --accent-bg-color: #ffd60a; }\n");

        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QColor("#ffd60a"));
        QCOMPARE(theme.accentForeground(), QColor("#000000"));
        QCOMPARE(theme.window(), QColor("#1e1e2e"));
    }

    // The sidebar's opacity is Skald's setting. A theme's old value is carried
    // over on the first launch only.
    void sidebarOpacityIsASettingThatAdoptsTheThemeOnce()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml", "background = \"#1e1e2e\"\n");
        write(dir.path() + "/theme/skald.css", ":root { --skald-sidebar-opacity: 0.7; }\n");
        {
            Theme theme(dir.path());
            QVERIFY(theme.transparentSidebar());
            QCOMPARE(theme.sidebarOpacitySetting(), 0.7);
            QCOMPARE(theme.sidebarOpacity(), 0.7);
            theme.setSidebarOpacitySetting(0.456);
            QCOMPARE(theme.sidebarOpacity(), 0.46);
        }
        // The theme no longer matters, and the setting survives a restart.
        write(dir.path() + "/theme/skald.css", ":root { --skald-sidebar-opacity: 0.2; }\n");
        {
            Theme theme(dir.path());
            QCOMPARE(theme.sidebarOpacity(), 0.46);
            theme.setTransparentSidebar(false);
            QCOMPARE(theme.sidebarOpacity(), 1.0);
            QCOMPARE(theme.sidebarOpacitySetting(), 0.46);
        }
        Theme theme(dir.path());
        QVERIFY(!theme.transparentSidebar());

        // Without a theme value the setting starts off, at 80%.
        QTemporaryDir fresh;
        Theme plain(fresh.path());
        QVERIFY(!plain.transparentSidebar());
        QCOMPARE(plain.sidebarOpacitySetting(), 0.8);
        QCOMPARE(plain.sidebarOpacity(), 1.0);
    }

    void sidebarOpacityReadsLikeGtk()
    {
        QCOMPARE(Theme::sidebarOpacityIn(""), 1.0);
        QCOMPARE(Theme::sidebarOpacityIn(":root { --skald-sidebar-opacity: 0.5 }"), 0.5);
        QCOMPARE(Theme::sidebarOpacityIn(":root { --skald-sidebar-opacity: 3; }"), 1.0);
        // The last declaration wins even when it isn't a number, which GTK reads as unset.
        QCOMPARE(Theme::sidebarOpacityIn("a { --skald-sidebar-opacity: 0.5; } b { --skald-sidebar-opacity: 80%; }"), 1.0);
        // Other properties that merely contain the name don't count.
        QCOMPARE(Theme::sidebarOpacityIn(":root { --skald-sidebar-opacity-x: 0.2; --x--skald-sidebar-opacity: 0.3; }"), 1.0);
    }

    void translatorFallsBackToEnglish()
    {
        QTemporaryDir dir;
        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("en"));
        QVERIFY(translator.translate("Main", "Save").isNull());
    }

    void translatorFollowsTheLanguageFile()
    {
        QTemporaryDir dir;
        write(dir.path() + "/language", "pt-BR\n");
        write(dir.path() + "/pt.json", R"({"Save": "Guardar", "Open": "Abrir"})");
        write(dir.path() + "/pt-BR.json", R"({"Save": "Salvar"})");

        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("pt-BR"));
        QCOMPARE(translator.translate("Main", "Save"), QString("Salvar"));
        QCOMPARE(translator.translate("Main", "Open"), QString("Abrir"));
        QVERIFY(translator.translate("Main", "Quit").isNull());
    }

    void bundledEnglishHasThePluralForms()
    {
        QTemporaryDir dir;
        Translator translator(dir.path() + "/language");
        QCOMPARE(translator.translate("BookGrid", "%n titles", nullptr, 1), QString("%n title"));
        QCOMPARE(translator.translate("BookGrid", "%n titles", nullptr, 228), QString("%n titles"));
    }

private:
    static void write(const QString &path, const QByteArray &contents)
    {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write(contents);
    }
};

QTEST_MAIN(SkaldTests)
#include "skald_tests.moc"
