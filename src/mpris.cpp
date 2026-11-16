#include "mpris.h"

#include <QDBusConnection>
#include <QDBusMessage>

#include "backend.h"

static const char *Path = "/org/mpris/MediaPlayer2";
static const qlonglong Microseconds = 1000000;

MprisPlayer::MprisPlayer(Backend *backend, QObject *parent) : QDBusAbstractAdaptor(parent), m_backend(backend) {
    connect(backend, &Backend::bookChanged, this, &MprisPlayer::announce);
    connect(backend, &Backend::playerChanged, this, &MprisPlayer::announce);
    connect(backend, &Backend::rateChanged, this, &MprisPlayer::announce);
}

bool MprisPlayer::hasBook() const { return !m_backend->asin().isEmpty(); }

QString MprisPlayer::playbackStatus() const {
    return !hasBook() ? "Stopped" : m_backend->playing() ? "Playing" : "Paused";
}

double MprisPlayer::rate() const { return m_backend->rate(); }

void MprisPlayer::setRate(double rate) {
    m_backend->stepRate(qRound((rate - m_backend->rate()) * 10));
}

qlonglong MprisPlayer::position() const { return qlonglong(m_backend->position() * Microseconds); }

QVariantMap MprisPlayer::metadata() const {
    if (!hasBook())
        return {{"mpris:trackid", QVariant::fromValue(QDBusObjectPath("/org/mpris/MediaPlayer2/TrackList/NoTrack"))}};
    QVariantMap map{{"mpris:trackid", QVariant::fromValue(QDBusObjectPath("/dev/skald/book"))},
                    {"xesam:title", m_backend->chapter().isEmpty() ? m_backend->title() : m_backend->chapter()},
                    {"xesam:album", m_backend->title()}};
    if (!m_backend->author().isEmpty())
        map.insert("xesam:artist", QStringList{m_backend->author()});
    if (!m_backend->cover().isEmpty())
        map.insert("mpris:artUrl", m_backend->cover());
    if (m_backend->duration() > 0)
        map.insert("mpris:length", qlonglong(m_backend->duration() * Microseconds));
    return map;
}

// MPRIS has no change signals of its own; clients listen for
// PropertiesChanged. The player reports twice a second, so only what
// actually differs is sent.
void MprisPlayer::announce() {
    const QVariantMap now{{"PlaybackStatus", playbackStatus()}, {"Rate", rate()}, {"Metadata", metadata()},
                          {"CanGoNext", hasBook()}, {"CanGoPrevious", hasBook()}, {"CanPlay", hasBook()},
                          {"CanPause", hasBook()}, {"CanSeek", hasBook()}};
    QVariantMap changed;
    for (auto it = now.begin(); it != now.end(); ++it)
        if (m_announced.value(it.key()) != it.value())
            changed.insert(it.key(), it.value());
    if (changed.isEmpty())
        return;
    m_announced = now;
    auto signal = QDBusMessage::createSignal(Path, "org.freedesktop.DBus.Properties", "PropertiesChanged");
    signal << "org.mpris.MediaPlayer2.Player" << changed << QStringList();
    QDBusConnection::sessionBus().send(signal);
}

void MprisPlayer::Next() { m_backend->nextChapter(); }
void MprisPlayer::Previous() { m_backend->previousChapter(); }
void MprisPlayer::Pause() { m_backend->pause(); }
void MprisPlayer::PlayPause() { m_backend->toggle(); }
void MprisPlayer::Stop() { m_backend->pause(); }
void MprisPlayer::Play() { m_backend->play(); }

void MprisPlayer::Seek(qlonglong offset) {
    m_backend->skip(double(offset) / Microseconds);
    emit Seeked(position() + offset);
}

void MprisPlayer::SetPosition(const QDBusObjectPath &, qlonglong position) {
    m_backend->skip(double(position) / Microseconds - m_backend->position());
    emit Seeked(position);
}

Mpris::Mpris(Backend *backend, QObject *parent) : QObject(parent) {
    new MprisRoot(this);
    new MprisPlayer(backend, this);
    auto bus = QDBusConnection::sessionBus();
    if (bus.registerService("org.mpris.MediaPlayer2.skald"))
        bus.registerObject(Path, this);
}
