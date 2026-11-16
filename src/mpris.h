#pragma once

#include <QDBusAbstractAdaptor>
#include <QDBusObjectPath>
#include <QStringList>
#include <QVariantMap>

class Backend;

// Puts Skald on the session bus as an MPRIS player, so media keys, the bar,
// and the lock screen can see and control what is playing. "Next" and
// "Previous" move by chapter, which is what those keys mean in a book.
class MprisRoot : public QDBusAbstractAdaptor {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.mpris.MediaPlayer2")
    Q_PROPERTY(bool CanQuit READ no CONSTANT)
    Q_PROPERTY(bool CanRaise READ no CONSTANT)
    Q_PROPERTY(bool HasTrackList READ no CONSTANT)
    Q_PROPERTY(QString Identity READ identity CONSTANT)
    Q_PROPERTY(QString DesktopEntry READ desktopEntry CONSTANT)
    Q_PROPERTY(QStringList SupportedUriSchemes READ none CONSTANT)
    Q_PROPERTY(QStringList SupportedMimeTypes READ none CONSTANT)

public:
    explicit MprisRoot(QObject *parent) : QDBusAbstractAdaptor(parent) {}
    bool no() const { return false; }
    QString identity() const { return "Skald"; }
    QString desktopEntry() const { return "skald"; }
    QStringList none() const { return {}; }

public slots:
    void Raise() {}
    void Quit() {}
};

class MprisPlayer : public QDBusAbstractAdaptor {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.mpris.MediaPlayer2.Player")
    Q_PROPERTY(QString PlaybackStatus READ playbackStatus)
    Q_PROPERTY(double Rate READ rate WRITE setRate)
    Q_PROPERTY(QVariantMap Metadata READ metadata)
    Q_PROPERTY(double Volume READ volume WRITE setVolume)
    Q_PROPERTY(qlonglong Position READ position)
    Q_PROPERTY(double MinimumRate READ minimumRate CONSTANT)
    Q_PROPERTY(double MaximumRate READ maximumRate CONSTANT)
    Q_PROPERTY(bool CanGoNext READ hasBook)
    Q_PROPERTY(bool CanGoPrevious READ hasBook)
    Q_PROPERTY(bool CanPlay READ hasBook)
    Q_PROPERTY(bool CanPause READ hasBook)
    Q_PROPERTY(bool CanSeek READ hasBook)
    Q_PROPERTY(bool CanControl READ yes CONSTANT)

public:
    MprisPlayer(Backend *backend, QObject *parent);

    QString playbackStatus() const;
    double rate() const;
    void setRate(double rate);
    QVariantMap metadata() const;
    double volume() const { return 1.0; }
    void setVolume(double) {}
    qlonglong position() const;
    double minimumRate() const { return 0.5; }
    double maximumRate() const { return 3.5; }
    bool hasBook() const;
    bool yes() const { return true; }

public slots:
    void Next();
    void Previous();
    void Pause();
    void PlayPause();
    void Stop();
    void Play();
    void Seek(qlonglong offset);
    void SetPosition(const QDBusObjectPath &track, qlonglong position);
    void OpenUri(const QString &) {}

signals:
    void Seeked(qlonglong position);

private:
    void announce();

    Backend *m_backend;
    QVariantMap m_announced;
};

// Registers both interfaces; a no-op when the session bus is not there.
class Mpris : public QObject {
    Q_OBJECT

public:
    explicit Mpris(Backend *backend, QObject *parent = nullptr);
};
