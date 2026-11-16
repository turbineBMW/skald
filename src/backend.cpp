#include "backend.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <QStandardPaths>
#include <QUrlQuery>
#include <cmath>

// How long a book may take to start before the window says so.
static const int StallMilliseconds = 25000;

Backend::Backend(const QString &directory, QObject *parent)
    : QObject(parent),
      m_directory(directory.isEmpty() ? QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation) + "/skald"
                                      : directory),
      m_settings(directory.isEmpty()
                     ? QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation) + "/skald/skald.ini"
                     : directory + "/skald.ini",
                 QSettings::IniFormat) {
    // Other marketplaces work by setting domain=www.audible.co.uk (and so on)
    // in ~/.config/skald/skald.ini.
    m_domain = m_settings.value("domain", "www.audible.com").toString();
    m_rate = qBound(0.5, m_settings.value("rate", 1.0).toDouble(), 3.5);

    // Audible streams with Widevine, which Qt WebEngine borrows from a
    // Chromium or Chrome profile. Without it nothing can play, so say so.
    if (directory.isEmpty()) {
        const QString home = QDir::homePath();
        if (findWidevine({home + "/.config/chromium/WidevineCdm", home + "/.config/google-chrome/WidevineCdm",
                          "/opt/google/chrome/WidevineCdm", "/usr/lib/chromium", "/usr/lib/chromium/WidevineCdm"})
                .isEmpty())
            m_widevineProblem = tr("Widevine was not found, so Audible cannot play. Open Chromium once and play "
                                   "something protected there, then start Skald again.");
    }
    loadCache();
}

QUrl Backend::libraryUrl() const {
    return QUrl("https://" + m_domain + "/library/titles?pageSize=50");
}

QUrl Backend::playerUrl(const QString &asin) const {
    QUrl url("https://" + m_domain + "/webplayer");
    QUrlQuery query;
    query.addQueryItem("asin", asin);
    url.setQuery(query);
    return url;
}

// Amazon's sign-in lives on amazon.* under /ap/, and audible bounces through
// its own /ap/ pages on the way there and back.
bool Backend::isSignIn(const QUrl &url) {
    const QString host = url.host(), path = url.path();
    return host.contains(".amazon.") || host.startsWith("amazon.") || path.startsWith("/ap/")
        || path.contains("signin") || path.contains("sign-in");
}

QString Backend::findWidevine(const QStringList &roots) {
    for (const auto &root : roots) {
        const QDir dir(root);
        if (QFileInfo::exists(dir.filePath("libwidevinecdm.so")))
            return dir.filePath("libwidevinecdm.so");
        // A browser profile keeps one directory per downloaded version.
        for (const auto &version : dir.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name | QDir::Reversed)) {
            const QString path = dir.filePath(version + "/_platform_specific/linux_x64/libwidevinecdm.so");
            if (QFileInfo::exists(path))
                return path;
        }
    }
    return {};
}

// The page names the current chapter but not its number. Titles can repeat
// ("Interlude"), so among the matches pick the one that starts nearest to
// where the audio says this chapter began.
int Backend::chapterIndexFor(const QVariantList &chapters, const QString &title, double chapterStart) {
    int best = -1;
    double bestDistance = 0, start = 0;
    for (int i = 0; i < chapters.size(); ++i) {
        const QVariantMap chapter = chapters.at(i).toMap();
        if (chapter.value("title").toString() == title) {
            const double distance = std::abs(start - chapterStart);
            if (best < 0 || distance < bestDistance) {
                best = i;
                bestDistance = distance;
            }
        }
        start += chapter.value("duration").toDouble();
    }
    return best;
}

QString Backend::clock(double seconds) {
    const qlonglong total = seconds > 0 && std::isfinite(seconds) ? qlonglong(seconds) : 0;
    const qlonglong h = total / 3600, m = total % 3600 / 60, s = total % 60;
    return h > 0 ? QString("%1:%2:%3").arg(h).arg(m, 2, 10, QChar('0')).arg(s, 2, 10, QChar('0'))
                 : QString("%1:%2").arg(m).arg(s, 2, 10, QChar('0'));
}

QString Backend::script(const QString &name) const {
    QFile file(":/web/" + name + ".js");
    return file.open(QIODevice::ReadOnly) ? QString::fromUtf8(file.readAll()) : QString();
}

QString Backend::lastAsin() const {
    return m_settings.value("lastAsin").toString();
}

void Backend::setFilter(const QString &filter) {
    if (filter == m_filter)
        return;
    m_filter = filter;
    m_library.setFilter(filter);
    emit filterChanged();
}

void Backend::setState(const QString &state) {
    if (state == m_state)
        return;
    m_state = state;
    emit stateChanged();
}

void Backend::start() {
    emit loadSession(libraryUrl());
}

void Backend::sessionNavigated(const QUrl &url, bool finished) {
    if (isSignIn(url)) {
        setState("signin");
        return;
    }
    if (!finished || url.host() != m_domain)
        return;
    if (url.path().startsWith("/library")) {
        const bool arrived = m_state != "ready";
        setState("ready");
        if (arrived)
            beginRefresh();
    } else if (m_state == "signin") {
        // Signed in but landed somewhere else on the site; go and check.
        emit loadSession(libraryUrl());
    }
}

void Backend::playerNavigated(const QUrl &url) {
    // A session that expired sends the player to sign in; hand that to the
    // session page, which is the one the user can see.
    if (!m_asin.isEmpty() && isSignIn(url)) {
        setState("signin");
        emit loadSession(libraryUrl());
    }
}

void Backend::beginRefresh() {
    m_refreshing = true;
    m_refreshLoaded = 0;
    m_libraryError.clear();
    emit refreshChanged();
    emit runInSession(script("library") + "\nwindow.__skaldLibrary.start();");
}

void Backend::refresh() {
    if (m_state == "ready" && !m_refreshing)
        beginRefresh();
}

void Backend::ingestLibrary(const QString &json) {
    if (!m_refreshing || json.isEmpty())
        return;
    const QJsonObject o = QJsonDocument::fromJson(json.toUtf8()).object();
    const QString status = o.value("status").toString();
    if (status == "loading") {
        if (o.value("loaded").toInt() != m_refreshLoaded) {
            m_refreshLoaded = o.value("loaded").toInt();
            emit refreshChanged();
        }
        return;
    }
    if (status == "done") {
        m_library.setBooks(Book::listFromJson(o.value("books").toArray()));
        saveCache();
    } else if (status == "signin") {
        setState("signin");
        emit loadSession(libraryUrl());
    } else if (status == "error") {
        m_libraryError = tr("Could not read your library from Audible (%1).").arg(o.value("error").toString());
    } else {
        return; // "idle": the page has not picked the request up yet.
    }
    m_refreshing = false;
    emit refreshChanged();
}

void Backend::signOut() {
    if (!m_asin.isEmpty()) {
        m_asin.clear();
        m_title.clear();
        m_author.clear();
        m_cover.clear();
        m_chapters.clear();
        m_ready = m_playing = m_stalled = false;
        emit bookChanged();
        emit chaptersChanged();
        emit playerChanged();
        emit loadPlayer(QUrl("about:blank"));
    }
    m_library.setBooks({});
    QFile::remove(m_directory + "/library.json");
    m_settings.remove("lastAsin");
    setState("starting");
    emit clearSession();
    emit loadSession(libraryUrl());
}

void Backend::openBook(const QString &asin) {
    if (asin.isEmpty() || asin == m_asin)
        return;
    m_asin = asin;
    const Book *book = m_library.find(asin);
    m_title = book ? book->title : QString();
    m_author = book ? book->authors.join(", ") : QString();
    m_cover = book ? book->cover : QString();
    m_chapters.clear();
    m_ready = m_playing = m_stalled = false;
    m_playerError.clear();
    m_chapter.clear();
    m_chapterIndex = -1;
    m_position = m_duration = m_chapterPosition = m_chapterDuration = 0;
    m_opened.start();
    m_settings.setValue("lastAsin", asin);
    emit bookChanged();
    emit chaptersChanged();
    emit playerChanged();
    emit loadPlayer(playerUrl(asin));
}

void Backend::ingestPlayer(const QString &json) {
    if (m_asin.isEmpty())
        return;
    const QJsonObject o = json.isEmpty() ? QJsonObject() : QJsonDocument::fromJson(json.toUtf8()).object();
    const bool onPlayer = o.value("path").toString().startsWith("/webplayer");

    if (o.contains("chapters")) {
        m_chapters = o.value("chapters").toArray().toVariantList();
        emit chaptersChanged();
    }

    m_ready = onPlayer && o.value("ready").toBool();
    m_playing = m_ready && o.value("playing").toBool();
    m_playerError = o.value("error").toString();
    if (m_ready) {
        m_stalled = false;
        m_position = o.value("position").toDouble();
        m_duration = o.value("duration").toDouble();
        m_chapterPosition = o.value("chapterPosition").toDouble();
        m_chapterDuration = o.value("chapterDuration").toDouble();
        m_chapter = o.value("chapter").toString();
        m_chapterIndex = chapterIndexFor(m_chapters, m_chapter, m_position - m_chapterPosition);

        // The page knows the book even when it was opened before the library loaded.
        const QString title = o.value("title").toString(), cover = o.value("cover").toString();
        if ((m_title.isEmpty() && !title.isEmpty()) || (m_cover.isEmpty() && !cover.isEmpty())) {
            if (m_title.isEmpty()) m_title = title;
            if (m_cover.isEmpty()) m_cover = cover;
            emit bookChanged();
        }
        // Each book starts at the page's own speed; bring it to the chosen one.
        if (std::abs(o.value("rate").toDouble(1.0) - m_rate) > 0.001)
            command("rate", QString::number(m_rate, 'f', 2));
    } else if (m_opened.isValid() && m_opened.elapsed() > StallMilliseconds) {
        m_stalled = true;
    }
    emit playerChanged();
}

void Backend::command(const QString &name, const QString &value) {
    if (m_asin.isEmpty())
        return;
    emit runInPlayer(QString("window.__skald && window.__skald.command('%1', %2);").arg(name, value));
}

void Backend::toggle() { command("toggle"); }
void Backend::play() { command("play"); }
void Backend::pause() { command("pause"); }
void Backend::skip(double seconds) { command("skip", QString::number(seconds, 'f', 1)); }
void Backend::seekChapter(double seconds) { command("seekChapter", QString::number(seconds, 'f', 1)); }
void Backend::previousChapter() { command("previousChapter"); }
void Backend::nextChapter() { command("nextChapter"); }
void Backend::jumpToChapter(int index) { command("chapter", QString::number(index)); }

void Backend::stepRate(int steps) {
    const double rate = qBound(0.5, std::round((m_rate + steps * 0.1) * 10) / 10, 3.5);
    if (rate == m_rate)
        return;
    m_rate = rate;
    m_settings.setValue("rate", rate);
    emit rateChanged();
    command("rate", QString::number(rate, 'f', 2));
}

void Backend::loadCache() {
    QFile file(m_directory + "/library.json");
    if (file.open(QIODevice::ReadOnly))
        m_library.setBooks(Book::listFromJson(QJsonDocument::fromJson(file.readAll()).array()));
}

// The last library is kept so the window has something to show at once on
// the next start, while the fresh one loads.
void Backend::saveCache() const {
    QDir().mkpath(m_directory);
    QSaveFile file(m_directory + "/library.json");
    if (!file.open(QIODevice::WriteOnly))
        return;
    file.write(QJsonDocument(Book::listToJson(m_library.books())).toJson(QJsonDocument::Compact));
    file.commit();
}
