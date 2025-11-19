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
    property color accentColor: "#1887f0"

    signal controlRequested(string action)
    signal settingsRequested()

    Layout.fillWidth: true
    Layout.preferredHeight: contentColumn.implicitHeight + 40
    radius: 20
    border.width: 2
    border.color: "#4f5978"
    color: "#ffffff"

    function fs(value) {
        return value * fontScale
    }

    function systemdSummary() {
        var state = (serviceStatus.active_state || "unknown").toUpperCase()
        var sub = serviceStatus.sub_state || ""
        return state + (sub ? " (" + sub + ")" : "")
    }

    function serviceHealthy() {
        const state = (serviceStatus.active_state || "").toLowerCase()
        return state === "active"
    }

    function getSyncthingSummary() {
        if (syncthingStatus.available) {
            return `Online (${syncthingStatus.version || "unknown"})`
        }
        return "Unavailable"
    }

    function capitalize(text) {
        if (!text || text.length === 0)
            return ""
        return text.charAt(0).toUpperCase() + text.slice(1)
    }

    function friendlyServiceState() {
        const active = (serviceStatus.active_state || "").toLowerCase()
        const sub = (serviceStatus.sub_state || "").toLowerCase()
        const primary = active ? capitalize(active) : "Unknown"
        if (sub && sub !== active && sub.length > 0) {
            return `${primary} (${capitalize(sub)})`
        }
        return primary
    }

    function friendlySyncthingState() {
        if (syncthingStatus.available) {
            const version = syncthingStatus.version
            return version ? `Connected (${version})` : "Connected"
        }
        return "Offline"
    }

    ColumnLayout {
        id: contentColumn
        anchors.fill: parent
        anchors.leftMargin: 28
        anchors.rightMargin: 28
        anchors.bottomMargin: 28
        anchors.topMargin: 18
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            spacing: 24

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    text: "Service status"
                    font.pointSize: fs(18)
                    font.bold: true
                    color: "#1a1e2d"
                }

                Rectangle {
                    radius: 18
                    height: 96
                    color: serviceHealthy() ? "#c2ddff" : "#ffd4b8"
                    border.width: 0
                    Layout.fillWidth: true

                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 36
                        text: friendlyServiceState()
                        font.pointSize: fs(18)
                        font.bold: true
                        color: "#112233"
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    text: "Syncthing API"
                    font.pointSize: fs(18)
                    font.bold: true
                    color: "#1a1e2d"
                }

                Rectangle {
                    radius: 18
                    height: 96
                    color: syncthingStatus.available ? "#c4f485" : "#f53636"
                    border.width: 0
                    Layout.fillWidth: true

        Text {
                        anchors.centerIn: parent
                        width: parent.width - 36
                        text: friendlySyncthingState()
                        font.pointSize: fs(18)
                        font.bold: true
                        color: "#112233"
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 2
            color: "#6a738d"
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            Repeater {
                model: [
                    { label: "Start", action: "start" },
                    { label: "Stop", action: "stop" },
                    { label: "Restart", action: "restart" }
                ]

                delegate: Rectangle {
                    required property var modelData
                    width: 150
                    height: 64
                    radius: 18
                    color: controlBusy ? "#cfd7eb" : accentColor
                    opacity: controlBusy ? 0.7 : 1
                    border.width: 0

                    Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        font.pointSize: fs(18)
                        font.bold: true
                        color: "#ffffff"
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
                width: 150
                height: 64
                radius: 18
                color: "#ffffff"
                border.width: 2
                border.color: "#a7b2ce"

                Text {
                    anchors.centerIn: parent
                    text: "Settings"
                    font.pointSize: fs(18)
                    font.bold: true
                    color: "#0f1c3f"
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: card.settingsRequested()
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: (installerStatus && installerStatus.installer_disabled) && installerAttentionRequired
            text: "Syncthing installer disabled in config. Please install manually."
            font.pointSize: fs(16)
            color: "#8a2e00"
        }
    }
}

