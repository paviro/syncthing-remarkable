import QtQuick 2.5
import QtQuick.Layouts 1.3

Item {
    id: installerPage

    property real fontScale: 1.0
    property var installerStatus: null
    property bool dismissable: true
    property bool isInstalling: installerStatus && installerStatus.in_progress

    signal installRequested()
    signal dismissRequested()

    anchors.fill: parent

    function fs(value) {
        return value * fontScale
    }

    function installerState() {
        const status = installerStatus || {}
        const binaryReady = !!status.binary_present
        const serviceReady = !!status.service_installed
        return { binaryReady: binaryReady, serviceReady: serviceReady }
    }

    function installerPrimaryText() {
        const state = installerState()
        if (!state.binaryReady && !state.serviceReady)
            return "Syncthing is not ready yet."
        if (state.binaryReady && !state.serviceReady)
            return "systemd service is missing."
        if (!state.binaryReady && state.serviceReady)
            return "Syncthing binary is missing."
        return "Syncthing is ready."
    }

    function installerSecondaryText() {
        const state = installerState()
        if (!state.binaryReady && !state.serviceReady)
            return "We can download the latest Syncthing release from GitHub and install a systemd service for you."
        if (state.binaryReady && !state.serviceReady)
            return "We detected the Syncthing binary on disk, but the systemd service was removed—perhaps by an OS update. Do you want to install the systemd service?"
        if (!state.binaryReady && state.serviceReady)
            return "The systemd service is still configured, but the Syncthing binary is missing — maybe due to an app update. Do you want to reinstall Syncthing?"
        return ""
    }

    function progressMessage() {
        return (installerStatus && installerStatus.progress_message) || ""
    }

    function errorMessage() {
        return (installerStatus && installerStatus.error) || ""
    }

    Rectangle {
        id: backgroundRect
        anchors.fill: parent
        anchors.margins: -32
        color: "#d8dce6"
    }

    Rectangle {
        id: card
        property int cardPadding: 36
        width: Math.min(parent.width - 32, 1024)
        height: cardContent.implicitHeight + card.cardPadding * 2
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.max(16, (parent.height - height) / 2)
        radius: 24
        border.width: 3
        border.color: "#8a92a8"
        color: "#ffffff"

        Column {
            id: cardContent
            anchors.fill: parent
            anchors.margins: card.cardPadding
            spacing: 28

            RowLayout {
                id: heroRow
                width: parent.width
                spacing: 28
                visible: card.width >= 640

                Image {
                    id: heroIconWide
                    source: "qrc:/icon.png"
                    width: 110
                    height: 110
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                    Layout.alignment: Qt.AlignTop
                    Layout.preferredWidth: width
                    Layout.preferredHeight: height
                }

                Column {
                    Layout.fillWidth: true
                    spacing: 6

                    Text {
                        text: "Install Syncthing"
                        font.pointSize: fs(36)
                        font.bold: true
                        wrapMode: Text.WordWrap
                        width: parent.width
                        color: "#000000"
                    }

                    Text {
                        text: "Syncthing and systemd service installer"
                        font.pointSize: fs(18)
                        color: "#2a2e38"
                        wrapMode: Text.WordWrap
                        width: parent.width
                    }
                }
            }

            Column {
                id: heroStack
                width: parent.width
                spacing: 16
                visible: !heroRow.visible

                Image {
                    source: "qrc:/icon.png"
                    width: 110
                    height: 110
                    anchors.horizontalCenter: parent.horizontalCenter
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                }

                Text {
                    text: "Install Syncthing"
                    font.pointSize: fs(36)
                    font.bold: true
                    wrapMode: Text.WordWrap
                    width: parent.width
                    color: "#000000"
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    text: "Download the latest Syncthing build and configure the background service in one tap."
                    font.pointSize: fs(18)
                    color: "#2a2e38"
                    wrapMode: Text.WordWrap
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Rectangle {
                width: parent.width
                height: statusContainer.height + 48
                radius: 18
                color: "#e8edf5"
                border.color: "#a0aac2"
                border.width: 2

                Column {
                    id: statusContainer
                    width: parent.width - 48
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.margins: 24
                    spacing: 18

                    Text {
                        text: "Current status"
                        font.pointSize: fs(20)
                        font.bold: true
                        color: "#000000"
                        width: parent.width
                    }

                    Column {
                        id: statusColumn
                        width: parent.width
                        spacing: 12

                        Rectangle {
                            id: binaryCard
                            width: parent.width
                            height: binaryContent.implicitHeight + 36
                            radius: 14
                            color: installerState().binaryReady ? "#bce874" : "#ffd9a8"
                            border.color: installerState().binaryReady ? "#6b9e2e" : "#b85f00"
                            border.width: 2

                            Column {
                                id: binaryContent
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 18
                                spacing: 4

                                Text {
                                    text: installerState().binaryReady ? "Binary ready" : "Binary missing"
                                    font.pointSize: fs(18)
                                    font.bold: true
                                    color: installerState().binaryReady ? "#2a3d0a" : "#6e3800"
                                    wrapMode: Text.WordWrap
                                    width: parent.width
                                }

                                Text {
                                    text: installerState().binaryReady ? "Syncthing executable found on the device." : "We will download the latest Syncthing binary."
                                    font.pointSize: fs(16)
                                    color: "#1a1d22"
                                    wrapMode: Text.WordWrap
                                    width: parent.width
                                }
                            }
                        }

                        Rectangle {
                            id: serviceCard
                            width: parent.width
                            height: serviceContent.implicitHeight + 36
                            radius: 14
                            color: installerState().serviceReady ? "#bce874" : "#ffcaca"
                            border.color: installerState().serviceReady ? "#6b9e2e" : "#b81c1c"
                            border.width: 2

                            Column {
                                id: serviceContent
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 18
                                spacing: 4

                                Text {
                                    text: installerState().serviceReady ? "Service configured" : "Service missing"
                                    font.pointSize: fs(18)
                                    font.bold: true
                                    color: installerState().serviceReady ? "#2a3d0a" : "#6e0a0a"
                                    wrapMode: Text.WordWrap
                                    width: parent.width
                                }

                                Text {
                                    text: installerState().serviceReady ? "systemd service is active." : "We will create and enable the Syncthing systemd service."
                                    font.pointSize: fs(16)
                                    color: "#1a1d22"
                                    wrapMode: Text.WordWrap
                                    width: parent.width
                                }
                            }
                        }
                    }
                }
            }

            Column {
                width: parent.width
                spacing: 8

                Text {
                    text: installerPrimaryText()
                    font.pointSize: fs(20)
                    font.bold: true
                    wrapMode: Text.WordWrap
                    width: parent.width
                    color: "#000000"
                }

                Text {
                    text: installerSecondaryText()
                    visible: installerSecondaryText().length > 0
                    font.pointSize: fs(18)
                    color: "#1a1d22"
                    wrapMode: Text.WordWrap
                    width: parent.width
                }
            }

            Row {
                width: parent.width
                spacing: 20
                anchors.horizontalCenter: parent.horizontalCenter

                Rectangle {
                    id: installButton
                    width: Math.max(220, Math.min(card.width * 0.45, 420))
                    height: 72
                    radius: 14
                    border.width: 0
                    color: installerPage.isInstalling ? "#a8cef0" : "#1887f0"

                    Text {
                        anchors.centerIn: parent
                        text: installerPage.isInstalling ? "Installing..." : "Install now"
                        font.pointSize: fs(20)
                        font.bold: true
                        color: "#ffffff"
                    }

                    MouseArea {
                        anchors.fill: parent
                        enabled: !installerPage.isInstalling
                        onClicked: installerPage.installRequested()
                    }
                }

                Rectangle {
                    width: Math.max(220, Math.min(card.width * 0.45, 420))
                    height: 72
                    radius: 14
                    border.width: 0
                    color: "#cc3333"
                    visible: dismissable

                    Text {
                        anchors.centerIn: parent
                        text: "Not now"
                        font.pointSize: fs(20)
                        font.bold: true
                        color: "#ffffff"
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: installerPage.dismissRequested()
                    }
                }
            }
        }
    }

    Rectangle {
        visible: progressMessage().length > 0
        width: Math.min(parent.width - 32, 1024)
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: card.bottom
        anchors.topMargin: 16
        radius: 12
        color: "#d9e3f5"
        border.color: "#7a92c2"
        border.width: 2
        height: progressText.implicitHeight + 36

        Text {
            id: progressText
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 18
            text: progressMessage()
            font.pointSize: fs(16)
            color: "#0a1a3d"
            wrapMode: Text.WordWrap
        }
    }

    Rectangle {
        visible: errorMessage().length > 0
        width: Math.min(parent.width - 32, 1024)
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: card.bottom
        anchors.topMargin: 16
        radius: 12
        color: "#ffcccc"
        border.color: "#cc3333"
        border.width: 2
        height: errorText.implicitHeight + 36

        Text {
            id: errorText
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 18
            text: errorMessage()
            font.pointSize: fs(16)
            color: "#6e0000"
            wrapMode: Text.WordWrap
        }
    }
}

