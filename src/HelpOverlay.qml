import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// Every shortcut, on "?".
Rectangle {
    id: root

    signal closed()
    signal signOut()

    color: "#cc000000"

    MouseArea { anchors.fill: parent; onClicked: root.closed() }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 40, 520)
        height: Math.min(parent.height - 40, content.implicitHeight + 48)
        radius: theme.radius
        color: "#1a1a1e"

        MouseArea { anchors.fill: parent }

        ColumnLayout {
            id: content
            anchors.fill: parent
            anchors.margins: 24
            spacing: 14

            Label { text: qsTr("Keyboard shortcuts"); font.pixelSize: 18; font.bold: true }

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
                    spacing: 20
                    Label { Layout.preferredWidth: 110; text: line.modelData[0]; color: theme.accent; font.bold: true }
                    Label { Layout.fillWidth: true; text: line.modelData[1]; wrapMode: Text.Wrap }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                Button { text: qsTr("Sign out"); flat: true; Material.roundedScale: theme.radius; onClicked: root.signOut() }
            }
        }
    }
}
