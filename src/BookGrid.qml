import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// The library: a wall of covers, searched with "/" and walked with the arrows.
Item {
    id: root

    signal opened(string asin)

    readonly property bool searching: search.activeFocus

    function focusSearch() {
        search.forceActiveFocus()
        search.selectAll()
    }
    function focusGrid() { grid.forceActiveFocus() }

    Component.onCompleted: focusGrid()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            TextField {
                id: search
                Layout.preferredWidth: 320
                Material.roundedScale: theme.radius
                placeholderText: qsTr("Search")
                onTextEdited: backend.filter = text
                Keys.onEscapePressed: { text = ""; backend.filter = ""; root.focusGrid() }
                Keys.onReturnPressed: root.focusGrid()
                Keys.onDownPressed: root.focusGrid()
            }
            Item { Layout.fillWidth: true }
            BusyIndicator {
                implicitWidth: 28
                implicitHeight: 28
                running: visible
                visible: backend.refreshing
            }
            Label {
                opacity: 0.6
                text: backend.refreshing
                      ? (backend.refreshLoaded > 0 ? qsTr("Reading your library… %1").arg(backend.refreshLoaded)
                                                   : qsTr("Reading your library…"))
                      : backend.filter !== "" ? qsTr("%1 of %2").arg(backend.library.count).arg(backend.library.total)
                                              : qsTr("%n titles", "", backend.library.total)
            }
        }

        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            focus: true
            model: backend.library
            keyNavigationEnabled: true
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}

            readonly property int columns: Math.max(2, Math.floor(width / 190))
            cellWidth: Math.floor(width / columns)
            cellHeight: cellWidth + 70

            Keys.onReturnPressed: root.opened(backend.library.asinAt(currentIndex))
            Keys.onEnterPressed: root.opened(backend.library.asinAt(currentIndex))

            // A search or a refresh rebuilds the list; keep a title selected,
            // and start on the book last listened to.
            property bool placed: false
            onCountChanged: {
                if (count === 0)
                    return
                if (!placed && backend.filter === "") {
                    placed = true
                    currentIndex = Math.max(0, backend.library.rowOf(backend.lastAsin()))
                    positionViewAtIndex(currentIndex, GridView.Center)
                } else if (currentIndex < 0 || currentIndex >= count) {
                    currentIndex = 0
                }
            }

            delegate: Item {
                id: cell
                required property int index
                required property string asin
                required property string title
                required property string authors
                required property string cover
                required property string remaining
                required property int percent
                required property bool finished

                width: grid.cellWidth
                height: grid.cellHeight

                readonly property bool current: GridView.isCurrentItem
                readonly property bool playing: backend.asin === asin

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 4
                    radius: theme.radius
                    color: cell.current ? "#1d1d22" : "transparent"
                    border.width: cell.current && grid.activeFocus ? 2 : 0
                    border.color: theme.accent
                }

                Column {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 6

                    Rectangle {
                        width: parent.width
                        height: width
                        radius: Math.min(4, theme.radius)
                        color: "#1a1a1e"
                        clip: true

                        Image {
                            anchors.fill: parent
                            source: cell.cover
                            sourceSize: Qt.size(400, 400)
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }
                        // Listening progress, along the bottom edge of the cover.
                        Rectangle {
                            anchors { left: parent.left; bottom: parent.bottom }
                            height: 4
                            width: parent.width * (cell.finished ? 100 : cell.percent) / 100
                            color: theme.accent
                        }
                    }
                    Label {
                        width: parent.width
                        text: cell.title
                        elide: Text.ElideRight
                        font.bold: true
                        color: cell.playing ? theme.accent : Material.foreground
                    }
                    Label {
                        width: parent.width
                        text: cell.finished ? qsTr("Finished") : cell.remaining !== "" ? cell.remaining : cell.authors
                        elide: Text.ElideRight
                        opacity: 0.6
                        font.pixelSize: 12
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        grid.currentIndex = cell.index
                        grid.forceActiveFocus()
                        root.opened(cell.asin)
                    }
                }
            }
        }
    }

    Label {
        anchors.centerIn: parent
        visible: backend.library.count === 0 && !backend.refreshing && backend.state === "ready"
        opacity: 0.6
        text: backend.filter !== "" ? qsTr("Nothing matches “%1”").arg(backend.filter) : qsTr("Your library is empty")
    }
}
