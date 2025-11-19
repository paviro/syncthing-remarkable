import QtQuick 2.5
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.3

Item {
    id: peersPanel

    property real fontScale: 1.0
    property var peers: []
    property var syncthingStatus: ({})
    property color accentColor: "#1887f0"
    property string expandedPeerKey: ""

    Layout.fillWidth: true
    Layout.fillHeight: true

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
            return "0.00%"
        var numeric = Number(value)
        if (!isFinite(numeric))
            numeric = 0
        if (numeric >= 100)
            numeric = 100
        else
            numeric = Math.floor(Math.max(0, numeric) * 100) / 100
        return numeric.toFixed(2) + "%"
    }

    function formatTimeAgo(value) {
        if (!value)
            return "unknown"
        var timestamp = Date.parse(value)
        if (isNaN(timestamp))
            return value
        var seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000))
        if (seconds < 60)
            return "just now"
        if (seconds < 3600)
            return `${Math.floor(seconds / 60)} min ago`
        if (seconds < 86400)
            return `${Math.floor(seconds / 3600)} h ago`
        return `${Math.floor(seconds / 86400)} d ago`
    }

    function peerStatusInfo(peer) {
        if (!peer)
            return ({ label: "Unknown", color: "#ffd2a0" })
        if (peer.paused)
            return ({ label: "Paused", color: "#cfd7eb" })
        var needBytes = Number(peer.need_bytes || 0)
        if (!peer.connected)
            return ({ label: "Offline", color: "#f76060" })
        if (needBytes > 0)
            return ({ label: "Syncing", color: "#ffd2a0" })
        return ({ label: "Up to date", color: "#c4f485" })
    }

    function peerKey(peer) {
        if (!peer)
            return ""
        return peer.id || peer.device_id || peer.name || ""
    }

    function isPeerExpanded(peer) {
        var key = peerKey(peer)
        return key !== "" && key === expandedPeerKey
    }

    function togglePeer(peer) {
        var key = peerKey(peer)
        if (!key)
            return
        expandedPeerKey = expandedPeerKey === key ? "" : key
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 0
        spacing: 18

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ListView {
                id: peerList
                anchors.fill: parent
                spacing: 16
                model: peers
                delegate: Rectangle {
                    id: peerCard
                    required property var modelData
                    width: peerList.width
                    implicitHeight: peerContent.implicitHeight + 32
                    radius: 20
                    border.width: 2
                    border.color: "#6c7898"
                    color: "#ffffff"
                    readonly property bool expanded: peersPanel.isPeerExpanded(modelData)

                    Column {
                        id: peerContent
                        anchors.margins: 20
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 16

                        Row {
                            id: peerHeader
                            width: parent.width
                            spacing: 12

                            Text {
                                width: Math.max(0, peerHeader.width - peerStatusBadge.width - peerHeader.spacing)
                                text: modelData.name || modelData.id
                                font.pointSize: fs(20)
                                font.bold: true
                                color: "#14203b"
                                elide: Text.ElideRight
                                wrapMode: Text.NoWrap
                            }

                            Rectangle {
                                id: peerStatusBadge
                                readonly property var badge: peersPanel.peerStatusInfo(modelData)
                                radius: 14
                                color: peerStatusBadge.badge.color
                                width: Math.max(130, badgeText.implicitWidth + 24)
                                height: 38

                                Text {
                                    id: badgeText
                                    anchors.centerIn: parent
                                    text: peerStatusBadge.badge.label
                                    font.pointSize: fs(16)
                                    color: "#1b2236"
                                }
                            }
                        }

                        Rectangle {
                            width: parent.width
                            height: 14
                            radius: 8
                            color: "#cbd3e4"

                            Rectangle {
                                anchors.left: parent.left
                                anchors.verticalCenter: parent.verticalCenter
                                height: parent.height
                                width: parent.width * Math.min(1, (modelData.completion || 0) / 100)
                                radius: 8
                                color: accentColor
                                opacity: (modelData.connected && !modelData.paused) ? 1.0 : 0.35
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Text {
                                text: `Progress ${modelData.completion !== undefined ? peersPanel.formatPercent(modelData.completion || 0) : "n/a"}`
                                font.pointSize: fs(16)
                                color: "#232a40"
                            }

                            Text {
                                text: "·"
                                font.pointSize: fs(16)
                                color: "#232a40"
                                visible: modelData.need_bytes !== undefined
                            }

                            Text {
                                text: modelData.need_bytes !== undefined ? `Pending ${peersPanel.formatBytes(modelData.need_bytes)}` : ""
                                font.pointSize: fs(16)
                                color: "#232a40"
                                visible: modelData.need_bytes !== undefined
                            }
                        }

                        Item {
                            width: parent.width
                            height: peerCard.expanded ? 0 : 0.5
                        }

                        Rectangle {
                            width: parent.width
                            height: peerCard.expanded ? 2 : 0
                            color: "#aeb8cf"
                            visible: peerCard.expanded
                        }

                        Column {
                            id: peerDetails
                            spacing: 12
                            visible: peerCard.expanded

                            Column {
                                spacing: 4

                                Text {
                                    text: modelData.address ? `Address ${modelData.address}` : (modelData.connected ? "" : `Last seen ${peersPanel.formatTimeAgo(modelData.last_seen)}`)
                                    font.pointSize: fs(14)
                                    color: "#2b3146"
                                    visible: !!modelData.address || !modelData.connected
                                }

                                Text {
                                    text: modelData.client_version ? `Client ${modelData.client_version}` : ""
                                    font.pointSize: fs(14)
                                    color: "#2b3146"
                                    visible: !!modelData.client_version
                                }
                            }

                            Column {
                                spacing: 4
                                visible: (modelData.folders || []).length > 0

                                Text {
                                    text: "Folder progress"
                                    font.pointSize: fs(16)
                                    font.bold: true
                                    color: "#111c34"
                                }

                                Repeater {
                                    model: (modelData.folders || []).slice(0, 4)
                                    delegate: Text {
                                        text: `${modelData.folder_label}: ${modelData.completion !== undefined ? peersPanel.formatPercent(modelData.completion || 0) : (modelData.need_bytes !== undefined ? peersPanel.formatBytes(modelData.need_bytes) + " pending" : "n/a")}`
                                        font.pointSize: fs(14)
                                        color: "#2b3146"
                                    }
                                }
                            }

                            Item {
                                width: parent.width
                                height: peerCard.expanded ? 8 : 0
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton
                        onClicked: peersPanel.togglePeer(modelData)
                    }
                }
            }
        }

        Rectangle {
            visible: peers.length === 0
            radius: 18
            Layout.fillWidth: true
            height: 84
            color: "#ffffff"
            border.color: "#6c7898"

            Text {
                anchors.centerIn: parent
                text: syncthingStatus.available ? "No peers have connected yet." : "Waiting for Syncthing to respond..."
                font.pointSize: fs(18)
                color: "#111c34"
            }
        }
    }
}

