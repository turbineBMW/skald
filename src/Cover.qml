import QtQuick
import QtQuick.Effects

// Cover art with rounded corners, on a placeholder while it loads.
Item {
    id: root

    property url source
    property int radius: 9
    property int size: 400
    property alias status: image.status

    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: Qt.alpha(theme.foreground, 0.08)
    }

    Image {
        id: image
        anchors.fill: parent
        source: root.source
        sourceSize: Qt.size(root.size, root.size)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }
    Rectangle {
        id: mask
        anchors.fill: parent
        radius: root.radius
        visible: false
        layer.enabled: true
    }
    MultiEffect {
        anchors.fill: parent
        source: image
        maskEnabled: true
        maskSource: mask
        maskThresholdMin: 0.5
        maskSpreadAtMin: 1.0
        visible: image.status === Image.Ready
    }
}
