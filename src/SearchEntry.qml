import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl

// A libadwaita search entry: a filled, rounded field with a search icon.
TextField {
    id: control

    implicitHeight: 34
    leftPadding: 9 + 16 + 6
    rightPadding: 9
    verticalAlignment: TextInput.AlignVCenter
    color: theme.foreground
    placeholderTextColor: Qt.alpha(theme.foreground, 0.55)
    selectionColor: Qt.alpha(theme.accent, 0.3)
    selectedTextColor: theme.foreground

    background: Rectangle {
        radius: 9
        color: Qt.alpha(theme.foreground, 0.10)
        border.width: control.activeFocus ? 2 : 0
        border.color: Qt.alpha(theme.accent, 0.5)

        IconImage {
            anchors { left: parent.left; leftMargin: 9; verticalCenter: parent.verticalCenter }
            name: "system-search-symbolic"
            sourceSize: Qt.size(16, 16)
            color: theme.foreground
            opacity: 0.55
        }
    }
}
