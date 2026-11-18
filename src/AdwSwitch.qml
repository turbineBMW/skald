import QtQuick
import QtQuick.Controls

// A libadwaita switch: a pill-shaped trough, in the accent while on, with a
// light round knob that slides across.
Switch {
    id: control

    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    padding: 0
    opacity: enabled ? 1 : 0.5
    // The indicator is the whole control, so size the control to it.
    implicitWidth: indicator.implicitWidth
    implicitHeight: indicator.implicitHeight

    indicator: Rectangle {
        implicitWidth: 48
        implicitHeight: 26
        x: control.leftPadding
        y: control.topPadding + control.availableHeight / 2 - height / 2
        radius: height / 2
        color: control.checked ? theme.accent : Qt.alpha(theme.foreground, control.hovered ? 0.2 : 0.15)
        Behavior on color { ColorAnimation { duration: 100 } }

        Rectangle {
            width: 20
            height: 20
            radius: 10
            y: 3
            x: control.checked ? parent.width - width - 3 : 3
            // libadwaita mixes the knob from white and the view background.
            color: Qt.tint(theme.window, Qt.rgba(1, 1, 1, 0.8))
            border.width: 1
            border.color: Qt.rgba(0, 0, 0.02, 0.10)
            Behavior on x { NumberAnimation { duration: 100 } }
        }
    }

    contentItem: Item {}
}
