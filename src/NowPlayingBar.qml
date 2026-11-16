import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Shown under the library while a book is loaded, so it stays one key away.
// A libadwaita bottom bar: the header bar's color, with a shade along the top.
Rectangle {
    id: root

    signal opened()

    implicitHeight: 64
    color: theme.headerbar

    Rectangle {
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 1
        color: theme.headerbarShade
    }

    TapHandler { onTapped: root.opened() }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        spacing: 12

        Cover {
            Layout.preferredWidth: 44
            Layout.preferredHeight: 44
            source: backend.cover
            size: 96
            radius: 6
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Label { Layout.fillWidth: true; text: backend.title; elide: Text.ElideRight; font.weight: Font.Bold }
            Label {
                Layout.fillWidth: true
                elide: Text.ElideRight
                opacity: 0.55
                font.pixelSize: 12
                text: backend.ready ? backend.chapter : backend.stalled ? qsTr("Audible's player has not started this book.") : qsTr("Opening…")
            }
        }
        Label {
            opacity: 0.55
            font.pixelSize: 12
            font.features: { "tnum": 1 }
            visible: backend.ready
            text: backend.clock(backend.chapterPosition) + " / " + backend.clock(backend.chapterDuration)
        }
        AdwButton {
            suggested: true
            circular: true
            implicitHeight: 42
            enabled: backend.ready
            iconName: backend.playing ? "media-playback-pause-symbolic" : "media-playback-start-symbolic"
            icon.source: backend.playing ? "qrc:/icons/pause.svg" : "qrc:/icons/play.svg"
            tip: backend.playing ? qsTr("Pause") : qsTr("Play")
            onClicked: backend.toggle()
        }
    }
}
