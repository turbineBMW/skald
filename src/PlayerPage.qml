import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The book being played, laid out like a libadwaita split view: the chapters
// in a sidebar on the start side, and the cover, where you are in the
// chapter and the controls beside it.
//
// The sidebar's background fades with the theme's --skald-sidebar-opacity
// (its text never does); the window paints nothing then, so the content side
// paints its own background.
Item {
    id: root

    property bool chaptersVisible: true
    signal back()
    signal showWeb()
    signal toggleChapters()

    readonly property double timeLeft: backend.rate > 0 ? Math.max(0, backend.duration - backend.position) / backend.rate : 0
    readonly property bool hasChapters: backend.chapters.length > 0
    readonly property bool roomForSidebar: width >= 860
    readonly property bool sidebarShown: chaptersVisible && hasChapters && roomForSidebar

    // The back button belongs in whichever header bar is first in the window.
    Component {
        id: backButton
        AdwButton {
            flat: true
            iconName: "go-previous-symbolic"
            icon.source: "qrc:/icons/back.svg"
            tip: qsTr("Library")
            onClicked: root.back()
        }
    }

    Rectangle {
        id: sidebar
        anchors { top: parent.top; bottom: parent.bottom; left: parent.left }
        width: Math.min(340, Math.max(260, root.width * 0.28))
        visible: root.sidebarShown
        color: Qt.alpha(Window.active ? theme.sidebar : theme.sidebarBackdrop, theme.sidebarOpacity)

        // The line between the panes.
        Rectangle {
            anchors { top: parent.top; bottom: parent.bottom; right: parent.right }
            width: 1
            color: theme.headerbarShade
        }

        HeaderBar {
            id: sidebarHeader
            anchors { top: parent.top; left: parent.left; right: parent.right; rightMargin: 1 }
            color: "transparent"
            shade: false
            title: qsTr("Chapters")
            start: [Loader { sourceComponent: backButton }]
        }

        ListView {
            id: list
            anchors { top: sidebarHeader.bottom; left: parent.left; right: parent.right; bottom: parent.bottom; rightMargin: 1 }
            topMargin: 6
            bottomMargin: 4
            clip: true
            model: backend.chapters
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}

            // Follow the book as it moves from chapter to chapter.
            readonly property int playingIndex: backend.chapterIndex
            onPlayingIndexChanged: if (playingIndex >= 0) positionViewAtIndex(playingIndex, ListView.Contain)
            onCountChanged: if (playingIndex >= 0) positionViewAtIndex(playingIndex, ListView.Center)
            onVisibleChanged: if (visible && playingIndex >= 0) positionViewAtIndex(playingIndex, ListView.Center)

            // A GTK navigation sidebar row.
            delegate: AbstractButton {
                id: row
                required property int index
                required property var modelData
                readonly property bool current: index === backend.chapterIndex

                width: ListView.view.width
                height: 38
                hoverEnabled: true
                focusPolicy: Qt.NoFocus
                onClicked: backend.jumpToChapter(index)

                background: Rectangle {
                    x: 6
                    width: parent.width - 12
                    height: 36
                    radius: 9
                    color: Qt.alpha(theme.foreground, row.current ? (row.hovered ? 0.13 : 0.10)
                                                                  : row.down ? 0.16 : row.hovered ? 0.07 : 0)
                }
                contentItem: Item {
                    RowLayout {
                        x: 14
                        width: parent.width - 28
                        height: 36
                        spacing: 10
                        Label {
                            Layout.fillWidth: true
                            text: row.modelData.title
                            elide: Text.ElideRight
                            font.weight: row.current ? Font.Bold : Font.Normal
                        }
                        Label {
                            text: backend.clock(row.modelData.duration)
                            opacity: 0.55
                            font.pixelSize: 12
                            font.features: { "tnum": 1 }
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        id: content
        anchors { top: parent.top; bottom: parent.bottom; right: parent.right; left: sidebar.visible ? sidebar.right : parent.left }
        color: theme.window

        HeaderBar {
            id: header
            anchors { top: parent.top; left: parent.left; right: parent.right }
            start: [
                Loader { active: !sidebar.visible; visible: active; sourceComponent: backButton },
                AdwButton {
                    flat: true
                    visible: root.hasChapters && root.roomForSidebar
                    checkable: true
                    checked: root.chaptersVisible
                    iconName: "sidebar-show-symbolic"
                    tip: qsTr("Show or hide the chapters")
                    onClicked: root.toggleChapters()
                }
            ]
            end: [CloseButton {}]
        }

        ColumnLayout {
            anchors { top: header.bottom; left: parent.left; right: parent.right; bottom: parent.bottom; margins: 24 }
            spacing: 6

            Item { Layout.fillHeight: true }

            Cover {
                id: art
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.max(140, Math.min(400, root.height - 440))
                Layout.preferredHeight: Layout.preferredWidth
                source: backend.cover
                size: 720
                radius: 12
                opacity: backend.ready ? 1 : 0.35

                Spinner {
                    anchors.centerIn: parent
                    width: 32
                    height: 32
                    visible: !backend.ready && !backend.stalled
                }
            }

            // title-2, then a dim author and the chapter as a heading.
            Label {
                Layout.fillWidth: true
                Layout.topMargin: 14
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                font.pixelSize: 20
                font.weight: Font.ExtraBold
                text: backend.title
            }
            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                opacity: 0.55
                text: backend.author
            }
            Label {
                Layout.fillWidth: true
                Layout.topMargin: 8
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                font.weight: Font.Bold
                text: backend.ready ? backend.chapter : backend.stalled ? "" : qsTr("Opening…")
            }

            // A stuck book is almost always the page asking for something;
            // offer the page rather than leaving a spinner.
            ColumnLayout {
                Layout.alignment: Qt.AlignHCenter
                visible: backend.stalled || backend.playerError !== ""
                spacing: 12
                Label {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.maximumWidth: 460
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    text: backend.playerError !== "" ? qsTr("Audible's player reported a problem: %1").arg(backend.playerError)
                                                     : qsTr("Audible's player has not started this book.")
                }
                AdwButton {
                    Layout.alignment: Qt.AlignHCenter
                    pill: true
                    text: qsTr("Show Audible's player")
                    onClicked: root.showWeb()
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.maximumWidth: 640
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 8
                spacing: 6

                Label {
                    Layout.minimumWidth: 56
                    opacity: 0.55
                    font.pixelSize: 12
                    font.features: { "tnum": 1 }
                    text: backend.clock(seek.pressed ? seek.value : backend.chapterPosition)
                }
                AdwSlider {
                    id: seek
                    Layout.fillWidth: true
                    enabled: backend.ready
                    from: 0
                    to: Math.max(1, backend.chapterDuration)

                    // After a drag the page takes a moment to report the new
                    // position; hold the handle where it was dropped meanwhile.
                    property real held: -1
                    Binding on value { when: !seek.pressed; value: seek.held >= 0 ? seek.held : backend.chapterPosition }
                    onPressedChanged: {
                        if (pressed)
                            return
                        held = value
                        backend.seekChapter(value)
                        release.restart()
                    }
                    Timer { id: release; interval: 1200; onTriggered: seek.held = -1 }
                }
                Label {
                    Layout.minimumWidth: 56
                    horizontalAlignment: Text.AlignRight
                    opacity: 0.55
                    font.pixelSize: 12
                    font.features: { "tnum": 1 }
                    text: "-" + backend.clock(Math.max(0, backend.chapterDuration - (seek.pressed ? seek.value : backend.chapterPosition)))
                }
            }
            Label {
                Layout.alignment: Qt.AlignHCenter
                opacity: 0.55
                font.pixelSize: 12
                visible: backend.ready
                text: {
                    const hours = Math.floor(root.timeLeft / 3600), minutes = Math.floor(root.timeLeft % 3600 / 60)
                    return hours > 0 ? qsTr("%1h %2m left").arg(hours).arg(minutes) : qsTr("%1m left").arg(minutes)
                }
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 8
                spacing: 12
                enabled: backend.ready

                AdwButton {
                    flat: true; circular: true; implicitHeight: 44; iconSize: 20
                    iconName: "media-skip-backward-symbolic"; icon.source: "qrc:/icons/previous.svg"
                    tip: qsTr("Previous chapter")
                    onClicked: backend.previousChapter()
                }
                AdwButton {
                    flat: true; circular: true; implicitHeight: 44; iconSize: 20
                    iconName: "media-seek-backward-symbolic"; icon.source: "qrc:/icons/rewind.svg"
                    tip: qsTr("Back 30 seconds")
                    onClicked: backend.skip(-30)
                }
                AdwButton {
                    suggested: true; circular: true; implicitHeight: 64; iconSize: 24
                    iconName: backend.playing ? "media-playback-pause-symbolic" : "media-playback-start-symbolic"
                    icon.source: backend.playing ? "qrc:/icons/pause.svg" : "qrc:/icons/play.svg"
                    tip: backend.playing ? qsTr("Pause") : qsTr("Play")
                    onClicked: backend.toggle()
                }
                AdwButton {
                    flat: true; circular: true; implicitHeight: 44; iconSize: 20
                    iconName: "media-seek-forward-symbolic"; icon.source: "qrc:/icons/forward.svg"
                    tip: qsTr("Forward 30 seconds")
                    onClicked: backend.skip(30)
                }
                AdwButton {
                    flat: true; circular: true; implicitHeight: 44; iconSize: 20
                    iconName: "media-skip-forward-symbolic"; icon.source: "qrc:/icons/next.svg"
                    tip: qsTr("Next chapter")
                    onClicked: backend.nextChapter()
                }
            }

            // Speed, as GTK's linked buttons.
            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 8
                implicitWidth: speed.implicitWidth
                implicitHeight: 34
                radius: 9
                color: Qt.alpha(theme.foreground, 0.10)

                RowLayout {
                    id: speed
                    anchors.fill: parent
                    spacing: 0
                    AdwButton { flat: true; iconName: "list-remove-symbolic"; tip: qsTr("Slower"); onClicked: backend.stepRate(-1) }
                    Label {
                        Layout.minimumWidth: 52
                        horizontalAlignment: Text.AlignHCenter
                        font.weight: Font.Bold
                        // Reads translator.language so the number re-renders on a language switch.
                        text: {
                            translator.language
                            return qsTr("%1×").arg(backend.rate.toLocaleString(Qt.locale(), 'f', 1))
                        }
                    }
                    AdwButton { flat: true; iconName: "list-add-symbolic"; tip: qsTr("Faster"); onClicked: backend.stepRate(1) }
                }
            }

            Item { Layout.fillHeight: true }
        }
    }
}
