import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The library: a wall of covers, searched with "/" and walked with the arrows.
Rectangle {
    id: root

    signal opened(string asin)
    // From the main menu.
    signal preferences()
    signal shortcuts()

    readonly property bool searching: search.activeFocus

    function focusSearch() {
        search.forceActiveFocus()
        search.selectAll()
    }
    function focusGrid() { grid.forceActiveFocus() }

    color: theme.window
    Component.onCompleted: focusGrid()

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        HeaderBar {
            Layout.fillWidth: true
            start: [
                AdwButton {
                    flat: true
                    iconName: "view-refresh-symbolic"
                    tip: qsTr("Read the library from Audible again")
                    enabled: !backend.refreshing
                    onClicked: backend.refresh()
                },
                Spinner { visible: backend.refreshing }
            ]
            center: SearchEntry {
                id: search
                anchors.centerIn: parent
                width: Math.min(parent.width, 360)
                placeholderText: qsTr("Search library")
                onTextEdited: backend.filter = text
                Keys.onEscapePressed: { text = ""; backend.filter = ""; root.focusGrid() }
                Keys.onReturnPressed: root.focusGrid()
                Keys.onDownPressed: root.focusGrid()
            }
            end: [
                AdwButton {
                    id: menuButton
                    flat: true
                    iconName: "open-menu-symbolic"
                    tip: qsTr("Main menu")
                    onClicked: mainMenu.open()

                    // A libadwaita popover menu under the button.
                    Popup {
                        id: mainMenu
                        y: menuButton.height + 4
                        x: menuButton.width - width
                        padding: 6
                        background: Rectangle {
                            radius: 12
                            color: theme.popover
                            border.width: 1
                            border.color: Qt.rgba(0, 0, 0.02, 0.10)
                        }
                        contentItem: Column {
                            spacing: 0
                            Repeater {
                                model: [
                                    [qsTr("Preferences"), function() { root.preferences() }],
                                    [qsTr("Keyboard Shortcuts"), function() { root.shortcuts() }],
                                ]
                                delegate: Rectangle {
                                    id: item
                                    required property var modelData
                                    width: Math.max(180, label.implicitWidth + 24)
                                    height: 34
                                    radius: 6
                                    color: hover.hovered ? Qt.alpha(theme.foreground, 0.07) : "transparent"
                                    Label { id: label; anchors.verticalCenter: parent.verticalCenter; x: 12; text: item.modelData[0] }
                                    HoverHandler { id: hover }
                                    TapHandler { onTapped: { mainMenu.close(); item.modelData[1]() } }
                                }
                            }
                        }
                    }
                },
                CloseButton {}
            ]
        }

        Label {
            Layout.fillWidth: true
            Layout.topMargin: 18
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            opacity: 0.55
            font.pixelSize: 13
            text: backend.refreshing
                  ? (backend.refreshLoaded > 0 ? qsTr("Reading your library… %1").arg(backend.refreshLoaded)
                                               : qsTr("Reading your library…"))
                  : backend.filter !== "" ? qsTr("%1 of %2").arg(backend.library.count).arg(backend.library.total)
                                          : qsTr("%n titles", "", backend.library.total)
        }

        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 18
            Layout.rightMargin: 18
            Layout.topMargin: 6
            clip: true
            focus: true
            model: backend.library
            keyNavigationEnabled: true
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}

            readonly property int columns: Math.max(2, Math.floor(width / 190))
            cellWidth: Math.floor(width / columns)
            cellHeight: cellWidth + 58

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

                // A GTK flow box child: a faint fill under the pointer and on
                // the selected title, and the focus ring while the keyboard
                // is in the grid.
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 3
                    radius: 12
                    color: Qt.alpha(theme.foreground, cell.current ? 0.10 : hover.hovered ? 0.07 : 0)
                    border.width: cell.current && grid.activeFocus ? 2 : 0
                    border.color: Qt.alpha(theme.accent, 0.5)
                }

                Column {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 4

                    Cover {
                        width: parent.width
                        height: width
                        source: cell.cover
                        radius: 9

                        // Listening progress, along the bottom of the cover.
                        Rectangle {
                            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 8 }
                            height: 4
                            radius: 2
                            visible: cell.percent > 0 && !cell.finished
                            color: Qt.rgba(0, 0, 0, 0.35)
                            Rectangle {
                                height: parent.height
                                radius: 2
                                width: parent.width * cell.percent / 100
                                color: theme.accent
                            }
                        }
                    }
                    Item { width: 1; height: 4 }
                    Label {
                        width: parent.width
                        text: cell.title
                        elide: Text.ElideRight
                        font.weight: Font.Bold
                        color: cell.playing ? theme.accent : theme.foreground
                    }
                    Label {
                        width: parent.width
                        text: cell.finished ? qsTr("Finished") : cell.remaining !== "" ? cell.remaining : cell.authors
                        elide: Text.ElideRight
                        opacity: 0.55
                        font.pixelSize: 12
                    }
                }

                HoverHandler { id: hover }
                TapHandler {
                    onTapped: {
                        grid.currentIndex = cell.index
                        grid.forceActiveFocus()
                        root.opened(cell.asin)
                    }
                }
            }
        }
    }

    // libadwaita's status page.
    Column {
        anchors.centerIn: parent
        visible: backend.library.count === 0 && !backend.refreshing && backend.state === "ready"
        spacing: 12
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            font.pixelSize: 26
            font.weight: Font.ExtraBold
            text: backend.filter !== "" ? qsTr("No Results Found") : qsTr("Your library is empty")
        }
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            opacity: 0.55
            visible: backend.filter !== ""
            text: qsTr("Nothing matches “%1”").arg(backend.filter)
        }
    }
}
