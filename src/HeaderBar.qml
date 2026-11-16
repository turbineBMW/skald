import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// A libadwaita header bar: buttons at the start and end, a bold title (or
// any widget) centered on the window, and a drag anywhere else moves the
// window. Over a sidebar it is drawn clear, so the sidebar shows through.
Item {
    id: bar

    property color color: theme.headerbar
    property bool shade: true
    property string title: ""
    property alias start: startRow.data
    property alias end: endRow.data
    property alias center: centerBox.data

    implicitHeight: 47

    Rectangle { anchors.fill: parent; color: bar.color }
    Rectangle {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: 1
        visible: bar.shade
        color: theme.headerbarShade
    }

    // Under the buttons, which keep their own clicks.
    DragHandler {
        target: null
        grabPermissions: PointerHandler.CanTakeOverFromHandlersOfDifferentType | PointerHandler.ApprovesTakeOverByAnything
        onActiveChanged: if (active) bar.Window.window.startSystemMove()
    }

    RowLayout {
        id: startRow
        anchors { left: parent.left; leftMargin: 7; verticalCenter: parent.verticalCenter; verticalCenterOffset: -0.5 }
        spacing: 6
    }
    RowLayout {
        id: endRow
        anchors { right: parent.right; rightMargin: 7; verticalCenter: parent.verticalCenter; verticalCenterOffset: -0.5 }
        spacing: 6
    }

    // Centered on the bar, and squeezed between the buttons when it can't be.
    readonly property real side: Math.max(startRow.width, endRow.width) + 13
    Item {
        id: centerBox
        anchors { verticalCenter: parent.verticalCenter; verticalCenterOffset: -0.5; horizontalCenter: parent.horizontalCenter }
        width: Math.max(0, bar.width - 2 * bar.side)
        height: parent.height
        visible: bar.title !== "" || children.length > 1

        Label {
            anchors.fill: parent
            visible: bar.title !== ""
            text: bar.title
            font.weight: Font.Bold
            elide: Text.ElideRight
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }
}
