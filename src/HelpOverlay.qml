import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Every shortcut, on "?" or the main menu, as a libadwaita dialog.
Rectangle {
    id: root

    signal closed()
    signal signOut()

    color: Qt.rgba(0, 0, 0.02, theme.dark ? 0.5 : 0.25)

    TapHandler { onTapped: root.closed() }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 40, 480)
        height: Math.min(parent.height - 40, content.implicitHeight + 47 + 24)
        radius: 15
        color: theme.popover
        border.width: 1
        border.color: Qt.rgba(0, 0, 0.02, 0.05)

        // Clicks inside the dialog don't close it.
        TapHandler {}

        HeaderBar {
            id: header
            anchors { top: parent.top; left: parent.left; right: parent.right; margins: 1 }
            color: "transparent"
            shade: false
            title: qsTr("Keyboard Shortcuts")
            end: [AdwButton { flat: true; circular: true; iconName: "window-close-symbolic"; tip: qsTr("Close"); onClicked: root.closed() }]
        }

        ColumnLayout {
            id: content
            anchors { top: header.bottom; left: parent.left; right: parent.right; margins: 24; topMargin: 6 }
            spacing: 10

            Repeater {
                model: [
                    ["Space", qsTr("Play or pause")],
                    ["← →", qsTr("Back or forward 30 seconds")],
                    ["Shift + ← →", qsTr("Previous or next chapter")],
                    ["− =", qsTr("Slower or faster")],
                    ["C", qsTr("Show or hide the chapters")],
                    ["/", qsTr("Search the library")],
                    ["Enter", qsTr("Open the selected book")],
                    ["P", qsTr("Go to the book that is playing")],
                    ["Esc", qsTr("Back to the library")],
                    ["R", qsTr("Read the library from Audible again")],
                    ["W", qsTr("Show or hide Audible's own player")],
                    ["?", qsTr("Show these shortcuts")],
                    ["Q", qsTr("Quit")],
                ]
                delegate: RowLayout {
                    id: line
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 16
                    Label { Layout.fillWidth: true; text: line.modelData[1]; wrapMode: Text.Wrap }
                    // A GTK shortcut label: the keys as a keycap.
                    Rectangle {
                        implicitWidth: key.implicitWidth + 16
                        implicitHeight: key.implicitHeight + 8
                        radius: 6
                        color: Qt.alpha(theme.foreground, 0.08)
                        border.width: 1
                        border.color: theme.border
                        Label { id: key; anchors.centerIn: parent; text: line.modelData[0]; font.pixelSize: 12; font.weight: Font.Bold }
                    }
                }
            }

            AdwButton {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 10
                pill: true
                text: qsTr("Sign Out")
                onClicked: root.signOut()
            }
        }
    }
}
