import QtQuick 2.5
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.3

Rectangle {
    id: foldersPanel

    property real fontScale: 1.0
    property var folders: []
    property var syncthingStatus: ({})

    Layout.fillWidth: true
    Layout.fillHeight: true
    radius: 8
    border.width: 2
    border.color: "black"
    color: "white"

    function fs(value) {
        return value * fontScale
    }

    function formatBytes(value) {
        if (value === undefined || value === null)
            return "n/a"
        var size = Number(value)
        var units = ["B", "KB", "MB", "GB", "TB"]
        var unitIndex = 0
        while (size >= 1024 && unitIndex < units.length - 1) {
            size = size / 1024
            unitIndex += 1
        }
        var precision = unitIndex === 0 ? 0 : 1
        return size.toFixed(precision) + " " + units[unitIndex]
    }

    function formatPercent(value) {
        if (value === undefined || value === null)
            return "0%"
        return value.toFixed(1) + "%"
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Text {
            text: "Folders"
            font.pointSize: fs(22)
            font.bold: true
        }

        ScrollView {
            id: folderScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ListView {
                id: folderList
                anchors.fill: parent
                spacing: 12
                model: folders
                delegate: Rectangle {
                    required property var modelData
                    width: folderList.width
                    implicitHeight: contentColumn.implicitHeight + 24
                    radius: 6
                    border.width: 1
                    border.color: "black"
                    color: "#fdfdfd"

                    Column {
                        id: contentColumn
                        anchors.margins: 12
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 6

                        Text {
                            text: modelData.label || modelData.id
                            font.pointSize: fs(18)
                            font.bold: true
                        }

                        Text {
                            text: `State: ${modelData.state || "unknown"}`
                            font.pointSize: fs(14)
                        }

                        Rectangle {
                            width: parent.width
                            height: 10
                            radius: 4
                            color: "#dddddd"

                            Rectangle {
                                anchors.left: parent.left
                                anchors.verticalCenter: parent.verticalCenter
                                height: parent.height
                                width: parent.width * ((modelData.completion || 0) / 100)
                                radius: 4
                                color: "#222222"
                            }
                        }

                        Text {
                            text: `Progress: ${foldersPanel.formatPercent(modelData.completion || 0)}`
                            font.pointSize: fs(14)
                        }

                        Text {
                            text: `Need ${foldersPanel.formatBytes(modelData.need_bytes)} of ${foldersPanel.formatBytes(modelData.global_bytes)}`
                            font.pointSize: fs(14)
                        }

                        Column {
                            spacing: 2
                            Text {
                                text: "Recent changes"
                                font.pointSize: fs(14)
                                font.bold: true
                            }
                            Repeater {
                                model: modelData.last_changes || []
                                delegate: Text {
                                    text: `${modelData.when} · ${modelData.action} · ${modelData.name}` + (modelData.origin ? ` (${modelData.origin})` : "")
                                    font.pointSize: fs(12)
                                }
                            }
                            Text {
                                visible: (modelData.last_changes || []).length === 0
                                text: "No recent changes"
                                font.pointSize: fs(12)
                                color: "#555555"
                            }
                        }
                    }
                }
            }
        }

        Text {
            visible: folders.length === 0
            text: syncthingStatus.available ? "No folders are configured." : "Waiting for Syncthing to respond..."
            font.pointSize: fs(16)
        }
    }
}

