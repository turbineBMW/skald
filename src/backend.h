#pragma once

#include <QElapsedTimer>
#include <QObject>
#include <QSettings>
#include <QUrl>
#include <QVariantList>

#include "library.h"

// Skald's state: the library, and what Audible's web player is doing. The
// two web pages live in QML; this class decides what they load and run, and
// turns what they report into properties.
class Backend : public QObject {
    Q_OBJECT
    // "starting" until the session is known, then "signin" or "ready".
    Q_PROPERTY(QString state READ state NOTIFY stateChanged)
    Q_PROPERTY(LibraryModel *library READ library CONSTANT)
    Q_PROPERTY(QString filter READ filter WRITE setFilter NOTIFY filterChanged)
    Q_PROPERTY(bool refreshing READ refreshing NOTIFY refreshChanged)
    Q_PROPERTY(int refreshLoaded READ refreshLoaded NOTIFY refreshChanged)
    Q_PROPERTY(QString libraryError READ libraryError NOTIFY refreshChanged)
    Q_PROPERTY(QString widevineProblem READ widevineProblem CONSTANT)

    Q_PROPERTY(QString asin READ asin NOTIFY bookChanged)
    Q_PROPERTY(QString title READ title NOTIFY bookChanged)
    Q_PROPERTY(QString author READ author NOTIFY bookChanged)
    Q_PROPERTY(QString cover READ cover NOTIFY bookChanged)
    Q_PROPERTY(QVariantList chapters READ chapters NOTIFY chaptersChanged)
    Q_PROPERTY(bool ready READ ready NOTIFY playerChanged)
    Q_PROPERTY(bool stalled READ stalled NOTIFY playerChanged)
    Q_PROPERTY(QString playerError READ playerError NOTIFY playerChanged)
    Q_PROPERTY(bool playing READ playing NOTIFY playerChanged)
    Q_PROPERTY(QString chapter READ chapter NOTIFY playerChanged)
    Q_PROPERTY(int chapterIndex READ chapterIndex NOTIFY playerChanged)
    Q_PROPERTY(double position READ position NOTIFY playerChanged)
    Q_PROPERTY(double duration READ duration NOTIFY playerChanged)
    Q_PROPERTY(double chapterPosition READ chapterPosition NOTIFY playerChanged)
    Q_PROPERTY(double chapterDuration READ chapterDuration NOTIFY playerChanged)
    Q_PROPERTY(double rate READ rate NOTIFY rateChanged)

public:
    // Tests pass a directory, which then holds the settings and the cache.
    explicit Backend(const QString &directory = {}, QObject *parent = nullptr);

    QString state() const { return m_state; }
    LibraryModel *library() { return &m_library; }
    QString filter() const { return m_filter; }
    void setFilter(const QString &filter);
    bool refreshing() const { return m_refreshing; }
    int refreshLoaded() const { return m_refreshLoaded; }
    QString libraryError() const { return m_libraryError; }
    QString widevineProblem() const { return m_widevineProblem; }

    QString asin() const { return m_asin; }
    QString title() const { return m_title; }
    QString author() const { return m_author; }
    QString cover() const { return m_cover; }
    QVariantList chapters() const { return m_chapters; }
    bool ready() const { return m_ready; }
    bool stalled() const { return m_stalled; }
    QString playerError() const { return m_playerError; }
    bool playing() const { return m_playing; }
    QString chapter() const { return m_chapter; }
    int chapterIndex() const { return m_chapterIndex; }
    double position() const { return m_position; }
    double duration() const { return m_duration; }
    double chapterPosition() const { return m_chapterPosition; }
    double chapterDuration() const { return m_chapterDuration; }
    double rate() const { return m_rate; }

    QUrl libraryUrl() const;
    QUrl playerUrl(const QString &asin) const;
    static bool isSignIn(const QUrl &url);
    static QString findWidevine(const QStringList &roots);
    static int chapterIndexFor(const QVariantList &chapters, const QString &title, double chapterStart);

    // 3723 seconds as "1:02:03"; under an hour as "2:03".
    Q_INVOKABLE static QString clock(double seconds);
    Q_INVOKABLE QString script(const QString &name) const;
    Q_INVOKABLE QString lastAsin() const;

    // Called by the window as the two pages move and report.
    Q_INVOKABLE void start();
    Q_INVOKABLE void sessionNavigated(const QUrl &url, bool finished);
    Q_INVOKABLE void playerNavigated(const QUrl &url);
    Q_INVOKABLE void ingestLibrary(const QString &json);
    Q_INVOKABLE void ingestPlayer(const QString &json);

    Q_INVOKABLE void refresh();
    Q_INVOKABLE void signOut();
    Q_INVOKABLE void openBook(const QString &asin);
    Q_INVOKABLE void toggle();
    Q_INVOKABLE void play();
    Q_INVOKABLE void pause();
    Q_INVOKABLE void skip(double seconds);
    Q_INVOKABLE void seekChapter(double seconds);
    Q_INVOKABLE void previousChapter();
    Q_INVOKABLE void nextChapter();
    Q_INVOKABLE void jumpToChapter(int index);
    Q_INVOKABLE void stepRate(int steps);

signals:
    void stateChanged();
    void filterChanged();
    void refreshChanged();
    void bookChanged();
    void chaptersChanged();
    void playerChanged();
    void rateChanged();

    // Requests to the pages, carried out by the window.
    void loadSession(const QUrl &url);
    void runInSession(const QString &script);
    void loadPlayer(const QUrl &url);
    void runInPlayer(const QString &script);
    void clearSession();

private:
    void setState(const QString &state);
    void beginRefresh();
    void command(const QString &name, const QString &value = "null");
    void loadCache();
    void saveCache() const;

    QString m_directory;
    QSettings m_settings;
    QString m_domain;
    QString m_state = "starting";
    LibraryModel m_library;
    QString m_filter;
    bool m_refreshing = false;
    int m_refreshLoaded = 0;
    QString m_libraryError;
    QString m_widevineProblem;

    QString m_asin, m_title, m_author, m_cover;
    QVariantList m_chapters;
    bool m_ready = false, m_stalled = false, m_playing = false;
    QString m_playerError, m_chapter;
    int m_chapterIndex = -1;
    double m_position = 0, m_duration = 0, m_chapterPosition = 0, m_chapterDuration = 0;
    double m_rate = 1.0;
    QElapsedTimer m_opened;
};
