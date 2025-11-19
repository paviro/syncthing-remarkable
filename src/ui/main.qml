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
    color: backgroundColor

    property var serviceStatus: ({})
    property var syncthingStatus: ({})
    property var folders: []
    property bool controlBusy: false
    property var installerStatus: null
    property real fontScale: 1.25
    property bool installerPromptDismissed: false
    property string guiAddress: ""
    property var updateCheckResult: null
    property var updateStatus: null
    property int updateRestartCountdown: 0
    property color backgroundColor: "#cbd1de"
    property color accentColor: "#1887f0"
    property color textColorPrimary: "#08122e"
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
                    guiAddress = payload.gui_address || ""
                } catch (err) {
                    console.warn("Failed to parse backend data", err)
                }
                break
            case 101:
                try {
                    const control = JSON.parse(contents)
                } catch (errControl) {
                    console.warn("Control response error", errControl)
                }
                controlBusy = false
                break
            case 102:
                try {
                    installerStatus = JSON.parse(contents)
                } catch (errInstaller) {
                    console.warn("Installer status error", errInstaller)
                }
                break
            case 103:
                try {
                    const guiAddressResult = JSON.parse(contents)
                } catch (errGuiAddress) {
                    console.warn("GUI address response error", errGuiAddress)
                }
                controlBusy = false
                break
            case 104:
                try {
                    updateCheckResult = JSON.parse(contents)
                } catch (errUpdate) {
                    console.warn("Update check error", errUpdate)
                }
                break
            case 105:
                try {
                    updateStatus = JSON.parse(contents)
                    if (updateStatus.restart_seconds_remaining !== undefined && updateStatus.restart_seconds_remaining !== null) {
                        updateRestartCountdown = updateStatus.restart_seconds_remaining
                        if (updateStatus.pending_restart && updateRestartCountdown > 0) {
                            restartCountdownTimer.start()
                        } else if (!updateStatus.pending_restart) {
                            restartCountdownTimer.stop()
                            updateRestartCountdown = 0
                        }
                    } else if (!updateStatus || !updateStatus.pending_restart) {
                        restartCountdownTimer.stop()
                        updateRestartCountdown = 0
                    }
                } catch (errUpdateStatus) {
                    console.warn("Update status error", errUpdateStatus)
                }
                break
            case 500:
                try {
                    const errorPayload = JSON.parse(contents)
                } catch (errBackend) {
                    console.warn("Backend error payload parse issue", errBackend)
                }
                controlBusy = false
                break
            default:
                console.warn("Unhandled backend message", type, contents)
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

    function toggleGuiAddress(address) {
        if (controlBusy)
            return
        controlBusy = true
        backend.sendMessage(4, JSON.stringify({ address: address }))
    }

    function checkForUpdates() {
        backend.sendMessage(5, JSON.stringify({}))
    }

    function downloadUpdate() {
        backend.sendMessage(6, JSON.stringify({}))
    }

    function requestRestart() {
        backend.sendMessage(7, JSON.stringify({}))
    }

    Timer {
        interval: 5000
        repeat: true
        running: true
        onTriggered: requestRefresh("timer")
    }

    Timer {
        id: restartCountdownTimer
        interval: 1000
        repeat: true
        onTriggered: {
            if (root.updateRestartCountdown > 0) {
                root.updateRestartCountdown--
            } else {
                stop()
            }
        }
    }

    Component.onCompleted: requestRefresh("initial")

    StackLayout {
        anchors.fill: parent
        currentIndex: canShowInstallerPrompt() ? 1 : 0

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 36
                spacing: 28

            HeaderBar {
                title: "Syncthing"
                fontScale: root.fontScale
                    accentColor: root.accentColor
                    titleColor: root.textColorPrimary
                
                onCloseRequested: {
                    root.unloading()
                    root.close()
                }
            }

            ServiceStatusCard {
                fontScale: root.fontScale
                serviceStatus: root.serviceStatus
                syncthingStatus: root.syncthingStatus
                controlBusy: root.controlBusy
                installerStatus: root.installerStatus
                installerAttentionRequired: root.installerNeedsAttention()
                Layout.fillWidth: true
                    accentColor: root.accentColor

                onControlRequested: controlService(action)
                onSettingsRequested: settingsOverlay.show()
            }

            FolderListPanel {
                fontScale: root.fontScale
                folders: root.folders
                syncthingStatus: root.syncthingStatus
                Layout.fillWidth: true
                Layout.fillHeight: true
                    accentColor: root.accentColor
            }

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
        accentColor: root.accentColor
        serviceStatus: root.serviceStatus
        controlBusy: root.controlBusy
        guiAddress: root.guiAddress
        updateCheckResult: root.updateCheckResult
        updateStatus: root.updateStatus
        updateRestartCountdown: root.updateRestartCountdown

        onCloseRequested: settingsOverlay.hide()
        
        onAutostartToggleRequested: function(enable) {
            controlService(enable ? "enable" : "disable")
        }

        onGuiAddressToggleRequested: function(address) {
            toggleGuiAddress(address)
        }

        onCheckForUpdatesRequested: function() {
            checkForUpdates()
        }

        onDownloadUpdateRequested: function() {
            downloadUpdate()
        }

        onRestartRequested: function() {
            requestRestart()
        }
    }
}
