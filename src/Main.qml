import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts
import QtWebEngine

ApplicationWindow {
    id: win
    width: 1180
    height: 780
    minimumWidth: 720
    minimumHeight: 520
    visible: true
    title: backend.title ? qsTr("%1 — Skald").arg(backend.title) : "Skald"
    color: "#0e0e10"

    Material.theme: Material.Dark
    Material.accent: theme.accent
    Material.roundedScale: theme.radius

    property bool helpVisible: false
    // Shows Audible's own player page, for when something there needs a click.
    property bool webVisible: false
    property bool chaptersVisible: true
    property string page: "library"
    property bool quitting: false

    readonly property bool signingIn: backend.state === "signin"
    // Single-key shortcuts stand down while the user is typing somewhere.
    readonly property bool keys: !signingIn && !webVisible && !library.searching
    readonly property bool onPlayer: page === "player" && backend.asin !== ""

    function openBook(asin) {
        if (asin === "")
            return
        backend.openBook(asin)
        page = "player"
    }

    // Pausing through the page lets Audible save the position before the
    // page goes away; quitting mid-playback could lose the last stretch.
    function quit() {
        if (quitting)
            return
        if (!backend.playing) {
            Qt.quit()
            return
        }
        quitting = true
        backend.pause()
        quitTimer.start()
    }

    function load(view, url) {
        if (view.url.toString() === url.toString())
            view.reload()
        else
            view.url = url
    }

    onClosing: function(close) {
        if (backend.playing && !quitting) {
            close.accepted = false
            quit()
        }
    }
    onPageChanged: if (page === "library") library.focusGrid()

    Timer { id: quitTimer; interval: 1200; onTriggered: Qt.quit() }

    Shortcut { sequence: "?"; context: Qt.ApplicationShortcut; enabled: win.keys; onActivated: win.helpVisible = !win.helpVisible }
    Shortcut { sequence: "Q"; context: Qt.ApplicationShortcut; enabled: win.keys; onActivated: win.quit() }
    Shortcut { sequence: "Space"; context: Qt.ApplicationShortcut; enabled: win.keys && backend.asin !== ""; onActivated: backend.toggle() }
    Shortcut { sequence: "Left"; context: Qt.ApplicationShortcut; enabled: win.keys && win.onPlayer; onActivated: backend.skip(-30) }
    Shortcut { sequence: "Right"; context: Qt.ApplicationShortcut; enabled: win.keys && win.onPlayer; onActivated: backend.skip(30) }
    Shortcut { sequence: "Shift+Left"; context: Qt.ApplicationShortcut; enabled: win.keys && win.onPlayer; onActivated: backend.previousChapter() }
    Shortcut { sequence: "Shift+Right"; context: Qt.ApplicationShortcut; enabled: win.keys && win.onPlayer; onActivated: backend.nextChapter() }
    Shortcut { sequences: ["-", "_"]; context: Qt.ApplicationShortcut; enabled: win.keys; onActivated: backend.stepRate(-1) }
    Shortcut { sequences: ["=", "+"]; context: Qt.ApplicationShortcut; enabled: win.keys; onActivated: backend.stepRate(1) }
    Shortcut { sequence: "C"; context: Qt.ApplicationShortcut; enabled: win.keys && win.onPlayer; onActivated: win.chaptersVisible = !win.chaptersVisible }
    Shortcut { sequence: "P"; context: Qt.ApplicationShortcut; enabled: win.keys && backend.asin !== ""; onActivated: win.page = "player" }
    Shortcut { sequence: "R"; context: Qt.ApplicationShortcut; enabled: win.keys; onActivated: backend.refresh() }
    Shortcut {
        sequence: "/"; context: Qt.ApplicationShortcut; enabled: win.keys
        onActivated: { win.helpVisible = false; win.page = "library"; library.focusSearch() }
    }
    Shortcut {
        sequence: "W"; context: Qt.ApplicationShortcut
        enabled: !win.signingIn && !library.searching && backend.asin !== ""
        onActivated: win.webVisible = !win.webVisible
    }
    Shortcut {
        sequence: "Escape"; context: Qt.ApplicationShortcut; enabled: !win.signingIn && !library.searching
        onActivated: {
            if (win.helpVisible) win.helpVisible = false
            else if (win.webVisible) win.webVisible = false
            else win.page = "library"
        }
    }

    Connections {
        target: backend
        function onLoadSession(url) { win.load(session, url) }
        function onRunInSession(script) { session.runJavaScript(script) }
        function onLoadPlayer(url) { win.load(player, url) }
        function onRunInPlayer(script) { player.runJavaScript(script) }
        function onStateChanged() { if (backend.state === "signin") win.webVisible = false }
    }

    // The player page reports twice a second. A fresh page has no bridge yet,
    // so the same poll installs it.
    Timer {
        interval: 500; repeat: true; running: backend.asin !== ""
        onTriggered: player.runJavaScript("window.__skald ? window.__skald.state() : 'install'", function(result) {
            if (result === "install") {
                player.runJavaScript(backend.script("player"))
                backend.ingestPlayer("")
            } else {
                backend.ingestPlayer(result || "")
            }
        })
    }
    Timer {
        interval: 400; repeat: true; running: backend.refreshing
        onTriggered: session.runJavaScript("window.__skaldLibrary ? window.__skaldLibrary.state() : ''",
                                           function(result) { backend.ingestLibrary(result || "") })
    }

    Component.onCompleted: backend.start()

    // Right-to-left languages mirror the window; the Translator sets the
    // application's layout direction.
    Item {
        anchors.fill: parent
        LayoutMirroring.enabled: Qt.application.layoutDirection === Qt.RightToLeft
        LayoutMirroring.childrenInherit: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                visible: notice.text !== ""
                implicitHeight: notice.implicitHeight + 20
                color: "#3a1d1d"
                Label {
                    id: notice
                    anchors.fill: parent
                    anchors.margins: 10
                    wrapMode: Text.Wrap
                    text: backend.widevineProblem !== "" ? backend.widevineProblem : backend.libraryError
                }
            }

            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: win.onPlayer ? 1 : 0

                BookGrid {
                    id: library
                    onOpened: function(asin) { win.openBook(asin) }
                }
                PlayerPage {
                    chaptersVisible: win.chaptersVisible
                    onShowWeb: win.webVisible = true
                    onBack: win.page = "library"
                }
            }

            NowPlayingBar {
                Layout.fillWidth: true
                visible: backend.asin !== "" && !win.onPlayer
                onOpened: win.page = "player"
            }
        }

        // Audible's player page. It does the playing whether or not it is shown.
        WebEngineView {
            id: player
            anchors.fill: parent
            z: 10
            visible: win.webVisible
            profile: webProfile
            settings.playbackRequiresUserGesture: false
            onUrlChanged: backend.playerNavigated(url)
            onNewWindowRequested: function(request) { request.openIn(player) }
        }

        // The session page: the Amazon sign-in when one is needed, and
        // otherwise an unseen audible page the library is read through.
        Rectangle {
            anchors.fill: parent
            z: 20
            visible: win.signingIn
            color: win.color

            Label {
                id: signInTitle
                anchors { top: parent.top; left: parent.left; right: parent.right; margins: 12 }
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Sign in to Audible to open your library")
                opacity: 0.8
            }
            WebEngineView {
                id: session
                anchors { top: signInTitle.bottom; left: parent.left; right: parent.right; bottom: parent.bottom; topMargin: 12 }
                profile: webProfile
                onUrlChanged: backend.sessionNavigated(url, false)
                onLoadingChanged: function(info) {
                    if (info.status === WebEngineView.LoadSucceededStatus)
                        backend.sessionNavigated(url, true)
                }
                onNewWindowRequested: function(request) { request.openIn(session) }
            }
        }

        BusyIndicator {
            anchors.centerIn: parent
            running: visible
            visible: backend.state === "starting" && backend.library.total === 0
        }

        HelpOverlay {
            anchors.fill: parent
            z: 30
            visible: win.helpVisible
            onClosed: win.helpVisible = false
            onSignOut: { win.helpVisible = false; win.page = "library"; backend.signOut() }
        }
    }
}
