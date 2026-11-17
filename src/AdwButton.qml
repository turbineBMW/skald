import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl

// A button drawn to libadwaita's measures, so Skald's controls look like
// those of the GTK apps beside it. Raised by default; also flat, circular,
// pill-shaped, and "suggested" (the accent-colored main action).
Button {
    id: control

    property bool suggested: false
    property bool circular: false
    property bool pill: false
    property string iconName: ""
    property int iconSize: 16
    // Shown on hover, as a GTK tooltip.
    property string tip: ""

    readonly property color ink: suggested ? theme.accentForeground : theme.foreground

    hoverEnabled: true
    // Clicks leave the keyboard where it was, so the single-key shortcuts
    // and the library's arrow keys keep working.
    focusPolicy: Qt.TabFocus
    font.weight: Font.Bold

    icon.name: iconName
    icon.width: iconSize
    icon.height: iconSize
    icon.color: ink

    padding: circular ? 0 : pill ? 10 : 5
    leftPadding: circular ? 0 : pill ? 32 : text === "" ? 9 : 10
    rightPadding: leftPadding
    implicitWidth: circular ? Math.max(34, implicitHeight)
                            : Math.max(34, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(34, implicitContentHeight + topPadding + bottomPadding)
    opacity: enabled ? 1 : 0.5

    contentItem: IconLabel {
        spacing: 6
        display: control.text === "" ? AbstractButton.IconOnly : control.icon.name === "" && control.icon.source.toString() === "" ? AbstractButton.TextOnly : AbstractButton.TextBesideIcon
        icon: control.icon
        text: control.text
        font: control.font
        color: control.ink
    }

    background: Rectangle {
        radius: control.circular || control.pill ? height / 2 : 9
        color: {
            if (control.suggested)
                return theme.accent
            const amount = control.flat ? (control.down || control.checked ? 0.16 : control.hovered ? 0.07 : 0)
                                        : (control.down || control.checked ? 0.30 : control.hovered ? 0.15 : 0.10)
            return Qt.alpha(theme.foreground, amount)
        }
        // The accent lightens or darkens under the pointer like a GTK button.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            visible: control.suggested
            color: Qt.alpha(theme.accentForeground, control.down ? 0.2 : control.hovered ? 0.1 : 0)
        }
        // Keyboard focus ring.
        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: parent.radius + 2
            color: "transparent"
            border.width: 2
            border.color: Qt.alpha(theme.accent, 0.5)
            visible: control.visualFocus
        }
    }

    ToolTip {
        visible: control.hovered && control.tip !== ""
        delay: 500
        text: control.tip
        padding: 0
        topPadding: 6; bottomPadding: 6; leftPadding: 10; rightPadding: 10
        contentItem: Text { text: control.tip; color: "white" }
        background: Rectangle { radius: 9; color: Qt.rgba(0, 0, 0, 0.8) }
    }
}
