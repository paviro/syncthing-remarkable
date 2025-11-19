import QtQuick 2.5
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.3
import net.asivery.AppLoad 1.0

Rectangle {
    id: root
    signal close
    function unloading() {
        if (backend && backend.terminate) {
            backend.terminate();
        }
    }

    anchors.fill: parent
    color: "white"

    property var serviceStatus: ({})
    property var syncthingStatus: ({})
    property var folders: []
    property string statusMessage: ""
    property string lastUpdated: ""
    property bool controlBusy: false
    property var installerStatus: null
    property real fontScale: 1.15
    property bool installerPromptDismissed: false
    onInstallerStatusChanged: {
        if (!installerNeedsAttention()) {
            installerPromptDismissed = false
        }
    }


    AppLoad {
        id: backend
        applicationID: "syncthing"
        onMessageReceived: function(type, contents) {
            if (type === 0 || contents === undefined)
                return
            switch (type) {
            case 100:
                try {
                    const payload = JSON.parse(contents)
                    serviceStatus = payload.systemd || {}
                    syncthingStatus = payload.syncthing || {}
                    folders = payload.folders || []
                    lastUpdated = payload.fetched_at || ""
                    if ((syncthingStatus.errors || []).length === 0 && payload.reason === "manual") {
                        statusMessage = "Refreshed"
                    }
                } catch (err) {
                    statusMessage = `Failed to parse backend data: ${err}`
                }
                break
            case 101:
                try {
                    const control = JSON.parse(contents)
                    statusMessage = control.message || "Control action completed"
                } catch (errControl) {
                    statusMessage = `Control response error: ${errControl}`
                }
                controlBusy = false
                break
            case 102:
                try {
                    installerStatus = JSON.parse(contents)
                } catch (errInstaller) {
                    statusMessage = `Installer status error: ${errInstaller}`
                }
                break
            case 500:
                try {
                    const errorPayload = JSON.parse(contents)
                    statusMessage = errorPayload.message || "Backend error"
                } catch (errBackend) {
                    statusMessage = `Backend error: ${errBackend}`
                }
                controlBusy = false
                break
            default:
                statusMessage = "Unhandled backend message"
                break
            }
        }
    }

    function requestRefresh(reason) {
        backend.sendMessage(1, JSON.stringify({ reason: reason || "manual" }))
    }

    function controlService(action) {
        if (controlBusy)
            return
        controlBusy = true
        backend.sendMessage(2, JSON.stringify({ action: action }))
    }

    function installerNeedsAttention() {
        if (!installerStatus)
            return false
        const binaryReady = !!installerStatus.binary_present
        const serviceReady = !!installerStatus.service_installed
        return !(binaryReady && serviceReady)
    }

    function canShowInstallerPrompt() {
        if (!installerStatus || installerStatus.installer_disabled || installerPromptDismissed)
            return false
        return installerNeedsAttention()
    }

    function triggerInstaller() {
        if (!installerStatus || installerStatus.in_progress)
            return
        backend.sendMessage(3, JSON.stringify({}))
    }

    Timer {
        interval: 5000
        repeat: true
        running: true
        onTriggered: requestRefresh("timer")
    }

    Component.onCompleted: requestRefresh("initial")

    StackLayout {
        anchors.fill: parent
        anchors.margins: 32
        currentIndex: canShowInstallerPrompt() ? 1 : 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 24

            HeaderBar {
                title: "Syncthing"
                lastUpdated: root.lastUpdated
                fontScale: root.fontScale
                
                onSettingsClicked: settingsOverlay.show()
            }

            ServiceStatusCard {
                fontScale: root.fontScale
                serviceStatus: root.serviceStatus
                syncthingStatus: root.syncthingStatus
                controlBusy: root.controlBusy
                installerStatus: root.installerStatus
                installerAttentionRequired: root.installerNeedsAttention()
                Layout.fillWidth: true

                onControlRequested: controlService(action)
                onRefreshRequested: requestRefresh(reason)
            }

            FolderListPanel {
                fontScale: root.fontScale
                folders: root.folders
                syncthingStatus: root.syncthingStatus
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            StatusFooter {
                fontScale: root.fontScale
                statusMessage: root.statusMessage
                controlBusy: root.controlBusy
            }
        }

        InstallerPage {
            Layout.fillWidth: true
            Layout.fillHeight: true
            fontScale: root.fontScale
            installerStatus: root.installerStatus
            dismissable: true

            onInstallRequested: triggerInstaller()
            onDismissRequested: {
                root.unloading()
                root.close()
            }
        }
    }

    SettingsOverlay {
        id: settingsOverlay
        anchors.fill: parent
        fontScale: root.fontScale
        serviceStatus: root.serviceStatus
        controlBusy: root.controlBusy

        onCloseRequested: settingsOverlay.hide()
        
        onAutostartToggleRequested: function(enable) {
            controlService(enable ? "enable" : "disable")
        }
    }
}
