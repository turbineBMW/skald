import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Preferences, from the main menu, as a libadwaita dialog with one boxed
// list per group. The settings live on `theme` (see theme.h), which keeps
// them in skald.ini.
Rectangle {
    id: root

    signal closed()

    color: Qt.rgba(0, 0, 0.02, theme.dark ? 0.5 : 0.25)

    TapHandler { onTapped: root.closed() }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 40, 520)
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
            title: qsTr("Preferences")
            end: [AdwButton { flat: true; circular: true; iconName: "window-close-symbolic"; tip: qsTr("Close"); onClicked: root.closed() }]
        }

        ColumnLayout {
            id: content
            anchors { top: header.bottom; left: parent.left; right: parent.right; margins: 24; topMargin: 6 }
            spacing: 12

            Label {
                text: qsTr("Appearance")
                font.weight: Font.Bold
            }

            // A boxed list: a card with a hairline between its rows.
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: rows.implicitHeight
                radius: 12
                color: theme.card
                border.width: 1
                border.color: theme.border

                ColumnLayout {
                    id: rows
                    anchors { left: parent.left; right: parent.right }
                    spacing: 0

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.margins: 12
                        Layout.leftMargin: 14
                        spacing: 12

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Label { text: qsTr("Transparent sidebar") }
                            Label {
                                Layout.fillWidth: true
                                text: qsTr("Show the desktop behind the chapters")
                                font.pixelSize: 12
                                opacity: 0.55
                                wrapMode: Text.Wrap
                            }
                        }
                        AdwSwitch {
                            checked: theme.transparentSidebar
                            onToggled: theme.transparentSidebar = checked
                        }
                    }

                    Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: theme.border }

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.margins: 12
                        Layout.leftMargin: 14
                        spacing: 12
                        // Its controls dim themselves while it is off.
                        enabled: theme.transparentSidebar

                        Label { Layout.fillWidth: true; text: qsTr("Sidebar opacity") }
                        Label {
                            text: qsTr("%1%").arg(Math.round(opacitySlider.value * 100))
                            opacity: 0.55
                        }
                        AdwSlider {
                            id: opacitySlider
                            Layout.preferredWidth: 180
                            from: 0
                            to: 1
                            stepSize: 0.05
                            value: theme.sidebarOpacitySetting
                            onMoved: theme.sidebarOpacitySetting = value
                        }
                    }
                }
            }
        }
    }
}
