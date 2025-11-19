import QtQuick 2.5
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.3

Rectangle {
    id: overlay
    anchors.fill: parent
    color: visible ? Qt.rgba(0, 0, 0, 0.2) : "transparent"
    visible: false
    z: 1000

    property real fontScale: 1.0
    property var serviceStatus: ({})
    property bool controlBusy: false

    signal closeRequested()
    signal autostartToggleRequested(bool enable)

    function fs(value) {
        return value * fontScale
    }

    function isAutostartEnabled() {
        const state = serviceStatus.unit_file_state || ""
        return state === "enabled" || state === "enabled-runtime"
    }

    MouseArea {
        anchors.fill: parent
        onClicked: overlay.closeRequested()
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width * 0.9, 800)
        height: Math.min(parent.height * 0.7, 500)
        color: "white"
        radius: 8
        border.color: "#000000"
        border.width: 3

        MouseArea {
            anchors.fill: parent
            onClicked: {} // Prevent clicks from propagating
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 40
            spacing: 28

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "Settings"
                    font.pointSize: fs(36)
                    font.bold: true
                    color: "#000000"
                }

                Item {
                    Layout.fillWidth: true
                }

                Button {
                    text: "Close"
                    font.pointSize: fs(20)
                    flat: true
                    onClicked: overlay.closeRequested()
                    
                    contentItem: Text {
                        text: parent.text
                        font: parent.font
                        color: "#000000"
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    
                    background: Rectangle {
                        color: parent.hovered ? "#cccccc" : "#e0e0e0"
                        radius: 4
                        border.color: "#000000"
                        border.width: 2
                        implicitWidth: 100
                        implicitHeight: 48
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 2
                color: "#000000"
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignHCenter
                spacing: 20

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 0
                    Layout.rightMargin: 0
                    spacing: 30

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 12

                        Text {
                            text: "Autostart Syncthing"
                            font.pointSize: fs(24)
                            font.bold: true
                            color: "#000000"
                        }

                        Text {
                            text: isAutostartEnabled() 
                                ? "Syncthing will start automatically when the device boots"
                                : "Syncthing must be started manually"
                            font.pointSize: fs(18)
                            color: "#333333"
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    Switch {
                        id: autostartSwitch
                        checked: isAutostartEnabled()
                        enabled: !controlBusy
                        scale: 3.0
                        Layout.alignment: Qt.AlignVCenter
                        Layout.rightMargin: 30
                        
                        onToggled: {
                            overlay.autostartToggleRequested(checked)
                        }
                    }
                }

                Text {
                    text: controlBusy ? "Applying changes..." : ""
                    font.pointSize: fs(18)
                    color: "#000000"
                    font.italic: true
                    visible: controlBusy
                    Layout.alignment: Qt.AlignHCenter
                }
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }

    function show() {
        visible = true
    }

    function hide() {
        visible = false
    }
}

