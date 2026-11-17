import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl

// The window's close button, as GTK draws it at the end of a header bar:
// a small round button inside a flat one.
AbstractButton {
    id: control

    implicitWidth: 34
    implicitHeight: 34
    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    onClicked: Window.window.close()

    contentItem: Item {
        Rectangle {
            anchors.centerIn: parent
            width: 24
            height: 24
            radius: 12
            color: Qt.alpha(theme.foreground, control.down ? 0.30 : control.hovered ? 0.15 : 0.10)
            IconImage {
                anchors.centerIn: parent
                name: "window-close-symbolic"
                sourceSize: Qt.size(16, 16)
                color: theme.foreground
            }
        }
    }
}
