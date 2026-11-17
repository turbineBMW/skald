import QtQuick
import QtQuick.Shapes

// libadwaita's spinner: a turning arc over a faint ring.
Item {
    id: root

    property bool running: visible
    property color color: theme.foreground

    implicitWidth: 16
    implicitHeight: 16

    readonly property real stroke: Math.max(2, width / 8)

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: Qt.alpha(root.color, 0.15)
            strokeWidth: root.stroke
            fillColor: "transparent"
            PathAngleArc {
                centerX: root.width / 2; centerY: root.height / 2
                radiusX: (root.width - root.stroke) / 2; radiusY: radiusX
                startAngle: 0; sweepAngle: 360
            }
        }
    }
    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        RotationAnimator on rotation {
            running: root.running
            from: 0; to: 360
            duration: 1100
            loops: Animation.Infinite
        }
        ShapePath {
            strokeColor: root.color
            strokeWidth: root.stroke
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc {
                centerX: root.width / 2; centerY: root.height / 2
                radiusX: (root.width - root.stroke) / 2; radiusY: radiusX
                startAngle: -90; sweepAngle: 100
            }
        }
    }
}
