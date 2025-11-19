import QtQuick 2.5
import QtQuick.Layouts 1.3

Rectangle {
    id: card

    property real fontScale: 1.0
    property var serviceStatus: ({})
    property var syncthingStatus: ({})
    property bool controlBusy: false
    property var installerStatus: null
    property bool installerAttentionRequired: false

    signal controlRequested(string action)
    signal refreshRequested(string reason)

    Layout.fillWidth: true
    Layout.preferredHeight: contentColumn.implicitHeight + 32
    radius: 8
    border.width: 2
    border.color: "black"
    color: "#f8f8f8"

    function fs(value) {
        return value * fontScale
    }

    function systemdSummary() {
        var state = (serviceStatus.active_state || "unknown").toUpperCase()
        var sub = serviceStatus.sub_state || ""
        return state + (sub ? " (" + sub + ")" : "")
    }

    ColumnLayout {
        id: contentColumn
        anchors.fill: parent
        anchors.margins: 16
        spacing: 8

        Text {
            text: `Service: ${card.systemdSummary()}`
            font.pointSize: fs(20)
        }

        Text {
            text: syncthingStatus.available ? `Syncthing: online (${syncthingStatus.version || "unknown"})` : "Syncthing: unavailable"
            font.pointSize: fs(16)
        }

        RowLayout {
            spacing: 12
            Layout.fillWidth: true

            Repeater {
                model: [
                    { label: "Start", action: "start" },
                    { label: "Stop", action: "stop" },
                    { label: "Restart", action: "restart" }
                ]
                delegate: Rectangle {
                    width: 140
                    height: 60
                    radius: 6
                    border.width: 2
                    border.color: "black"
                    color: controlBusy ? "#dddddd" : "white"
                    opacity: controlBusy ? 0.6 : 1

                    Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        font.pointSize: fs(16)
                    }

                    MouseArea {
                        anchors.fill: parent
                        enabled: !controlBusy
                        onClicked: card.controlRequested(modelData.action)
                    }
                }
            }

            Item { Layout.fillWidth: true }

            Rectangle {
                width: 160
                height: 60
                radius: 6
                border.width: 2
                border.color: "black"
                color: "white"

                Text {
                    anchors.centerIn: parent
                    text: "Refresh"
                    font.pointSize: fs(16)
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: card.refreshRequested("manual")
                }
            }
        }

        Repeater {
            model: syncthingStatus.errors || []
            delegate: Text {
                text: `[!] ${modelData}`
                font.pointSize: fs(14)
            }
        }

        Text {
            visible: (installerStatus && installerStatus.installer_disabled) && installerAttentionRequired
            text: "Syncthing installer disabled in config. Please install manually."
            font.pointSize: fs(14)
            color: "#bb4400"
        }
    }
}

