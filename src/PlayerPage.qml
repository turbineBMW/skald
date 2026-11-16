import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// The book being played: cover, where you are in the chapter, and the chapters.
Item {
    id: root

    property bool chaptersVisible: true
    signal back()
    signal showWeb()

    readonly property double timeLeft: backend.rate > 0 ? Math.max(0, backend.duration - backend.position) / backend.rate : 0

    RowLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 24

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            ToolButton {
                icon.source: "qrc:/icons/back.svg"
                icon.color: "#e8e8ea"
                text: qsTr("Library")
                Material.roundedScale: theme.radius
                onClicked: root.back()
            }

            Item { Layout.fillHeight: true }

            Rectangle {
                id: art
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.max(140, Math.min(360, root.height - 420))
                Layout.preferredHeight: Layout.preferredWidth
                radius: Math.min(4, theme.radius)
                color: "#1a1a1e"
                clip: true

                Image {
                    anchors.fill: parent
                    source: backend.cover
                    sourceSize: Qt.size(720, 720)
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    opacity: backend.ready ? 1 : 0.35
                }
                BusyIndicator {
                    anchors.centerIn: parent
                    running: visible
                    visible: !backend.ready && !backend.stalled
                }
            }

            Label {
                Layout.fillWidth: true
                Layout.topMargin: 8
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                font.pixelSize: 22
                font.bold: true
                text: backend.title
            }
            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                opacity: 0.65
                text: backend.author
            }
            Label {
                Layout.fillWidth: true
                Layout.topMargin: 6
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                color: theme.accent
                text: backend.ready ? backend.chapter : backend.stalled ? "" : qsTr("Opening…")
            }

            // A stuck book is almost always the page asking for something;
            // offer the page rather than leaving a spinner.
            ColumnLayout {
                Layout.alignment: Qt.AlignHCenter
                visible: backend.stalled || backend.playerError !== ""
                spacing: 8
                Label {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.maximumWidth: 460
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    text: backend.playerError !== "" ? qsTr("Audible's player reported a problem: %1").arg(backend.playerError)
                                                     : qsTr("Audible's player has not started this book.")
                }
                Button {
                    Layout.alignment: Qt.AlignHCenter
                    Material.roundedScale: theme.radius
                    text: qsTr("Show Audible's player")
                    onClicked: root.showWeb()
                }
            }

            Slider {
                id: seek
                Layout.fillWidth: true
                Layout.maximumWidth: 720
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 6
                enabled: backend.ready
                from: 0
                to: Math.max(1, backend.chapterDuration)

                // After a drag the page takes a moment to report the new
                // position; hold the handle where it was dropped meanwhile.
                property real held: -1
                Binding on value { when: !seek.pressed; value: seek.held >= 0 ? seek.held : backend.chapterPosition }
                onPressedChanged: {
                    if (pressed)
                        return
                    held = value
                    backend.seekChapter(value)
                    release.restart()
                }
                Timer { id: release; interval: 1200; onTriggered: seek.held = -1 }
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.maximumWidth: 720
                Layout.alignment: Qt.AlignHCenter
                Label { opacity: 0.7; text: backend.clock(seek.pressed ? seek.value : backend.chapterPosition) }
                Item { Layout.fillWidth: true }
                Label {
                    opacity: 0.7
                    visible: backend.ready
                    text: {
                        const hours = Math.floor(root.timeLeft / 3600), minutes = Math.floor(root.timeLeft % 3600 / 60)
                        return hours > 0 ? qsTr("%1h %2m left").arg(hours).arg(minutes) : qsTr("%1m left").arg(minutes)
                    }
                }
                Item { Layout.fillWidth: true }
                Label { opacity: 0.7; text: "-" + backend.clock(Math.max(0, backend.chapterDuration - (seek.pressed ? seek.value : backend.chapterPosition))) }
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 10
                enabled: backend.ready

                ToolButton {
                    icon.source: "qrc:/icons/previous.svg"; icon.color: "#e8e8ea"; icon.width: 30; icon.height: 30
                    onClicked: backend.previousChapter()
                    ToolTip.visible: hovered; ToolTip.text: qsTr("Previous chapter")
                }
                ToolButton {
                    icon.source: "qrc:/icons/rewind.svg"; icon.color: "#e8e8ea"; icon.width: 34; icon.height: 34
                    onClicked: backend.skip(-30)
                    ToolTip.visible: hovered; ToolTip.text: qsTr("Back 30 seconds")
                    Label { anchors.centerIn: parent; anchors.verticalCenterOffset: 2; text: "30"; font.pixelSize: 9; font.bold: true }
                }
                RoundButton {
                    implicitWidth: 68
                    implicitHeight: 68
                    // A true circle in the accent, whatever the window rounding is.
                    background: Rectangle { radius: width / 2; color: theme.accent; opacity: parent.enabled ? (parent.down ? 0.8 : 1) : 0.35 }
                    icon.source: backend.playing ? "qrc:/icons/pause.svg" : "qrc:/icons/play.svg"
                    icon.color: theme.accentForeground
                    icon.width: 32
                    icon.height: 32
                    onClicked: backend.toggle()
                }
                ToolButton {
                    icon.source: "qrc:/icons/forward.svg"; icon.color: "#e8e8ea"; icon.width: 34; icon.height: 34
                    onClicked: backend.skip(30)
                    ToolTip.visible: hovered; ToolTip.text: qsTr("Forward 30 seconds")
                    Label { anchors.centerIn: parent; anchors.verticalCenterOffset: 2; text: "30"; font.pixelSize: 9; font.bold: true }
                }
                ToolButton {
                    icon.source: "qrc:/icons/next.svg"; icon.color: "#e8e8ea"; icon.width: 30; icon.height: 30
                    onClicked: backend.nextChapter()
                    ToolTip.visible: hovered; ToolTip.text: qsTr("Next chapter")
                }
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 4
                ToolButton { text: "−"; Material.roundedScale: theme.radius; onClicked: backend.stepRate(-1) }
                Label {
                    Layout.minimumWidth: 56
                    horizontalAlignment: Text.AlignHCenter
                    // Reads translator.language so the number re-renders on a language switch.
                    text: {
                        translator.language
                        return qsTr("%1×").arg(backend.rate.toLocaleString(Qt.locale(), 'f', 1))
                    }
                }
                ToolButton { text: "+"; Material.roundedScale: theme.radius; onClicked: backend.stepRate(1) }
            }

            Item { Layout.fillHeight: true }
        }

        Rectangle {
            Layout.preferredWidth: 320
            Layout.fillHeight: true
            visible: root.chaptersVisible && backend.chapters.length > 0 && root.width >= 900
            radius: theme.radius
            color: "#151518"

            ListView {
                id: list
                anchors.fill: parent
                anchors.margins: 6
                clip: true
                model: backend.chapters
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}

                // Follow the book as it moves from chapter to chapter.
                readonly property int playingIndex: backend.chapterIndex
                onPlayingIndexChanged: if (playingIndex >= 0) positionViewAtIndex(playingIndex, ListView.Contain)
                onCountChanged: if (playingIndex >= 0) positionViewAtIndex(playingIndex, ListView.Center)

                delegate: ItemDelegate {
                    id: row
                    required property int index
                    required property var modelData
                    readonly property bool current: index === backend.chapterIndex

                    width: ListView.view.width
                    Material.roundedScale: Math.min(4, theme.radius)
                    onClicked: backend.jumpToChapter(index)

                    contentItem: RowLayout {
                        spacing: 10
                        Label {
                            Layout.fillWidth: true
                            text: row.modelData.title
                            elide: Text.ElideRight
                            color: row.current ? theme.accent : Material.foreground
                            font.bold: row.current
                        }
                        Label { text: backend.clock(row.modelData.duration); opacity: 0.55; font.pixelSize: 12 }
                    }
                }
            }
        }
    }
}
