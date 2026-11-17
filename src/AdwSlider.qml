import QtQuick
import QtQuick.Controls

// A libadwaita scale: a thin rounded trough, filled in the accent, with a
// light round knob.
Slider {
    id: control

    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    padding: 12
    opacity: enabled ? 1 : 0.5

    background: Rectangle {
        x: control.leftPadding
        y: control.topPadding + control.availableHeight / 2 - height / 2
        width: control.availableWidth
        height: 6
        radius: 3
        color: Qt.alpha(theme.foreground, 0.15)

        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: 3
            color: theme.accent
        }
    }

    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + control.availableHeight / 2 - height / 2
        width: 20
        height: 20
        radius: 10
        // libadwaita mixes the knob from white and the view background.
        color: Qt.tint(theme.window, Qt.rgba(1, 1, 1, 0.8))
        border.width: 1
        border.color: Qt.rgba(0, 0, 0.02, 0.10)
        scale: control.pressed ? 1.1 : 1
        Behavior on scale { NumberAnimation { duration: 120 } }
    }
}
