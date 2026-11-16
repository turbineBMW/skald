import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// Shown under the library while a book is loaded, so it stays one key away.
Rectangle {
    id: root

    signal opened()

    implicitHeight: 68
    color: "#151518"

    MouseArea { anchors.fill: parent; onClicked: root.opened() }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        spacing: 14

        Rectangle {
            Layout.preferredWidth: 48
            Layout.preferredHeight: 48
            radius: Math.min(4, theme.radius)
            color: "#1a1a1e"
            clip: true
            Image {
                anchors.fill: parent
                source: backend.cover
                sourceSize: Qt.size(96, 96)
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Label { Layout.fillWidth: true; text: backend.title; elide: Text.ElideRight; font.bold: true }
            Label {
                Layout.fillWidth: true
                elide: Text.ElideRight
                opacity: 0.6
                font.pixelSize: 12
                text: backend.ready ? backend.chapter : backend.stalled ? qsTr("Audible's player has not started this book.") : qsTr("Opening…")
            }
        }
        Label {
            opacity: 0.6
            visible: backend.ready
            text: backend.clock(backend.chapterPosition) + " / " + backend.clock(backend.chapterDuration)
        }
        RoundButton {
            implicitWidth: 48
            implicitHeight: 48
            enabled: backend.ready
            // A true circle in the accent, whatever the window rounding is.
            background: Rectangle { radius: width / 2; color: theme.accent; opacity: parent.enabled ? (parent.down ? 0.8 : 1) : 0.35 }
            icon.source: backend.playing ? "qrc:/icons/pause.svg" : "qrc:/icons/play.svg"
            icon.color: theme.accentForeground
            onClicked: backend.toggle()
        }
    }
}
