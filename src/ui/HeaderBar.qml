import QtQuick 2.5
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.3

Rectangle {
    property string title: "Syncthing"
    property real fontScale: 1.0
    property color accentColor: "#1887f0"
    property color titleColor: "#0a1a3d"

    signal closeRequested()

    Layout.fillWidth: true
    radius: 22
    border.width: 2
    border.color: "#4c5878"
    color: "#eef0f4"
    implicitHeight: contentRow.implicitHeight + 32

    function fs(value) {
        return value * fontScale
    }

    RowLayout {
        id: contentRow
        anchors.fill: parent
        anchors.margins: 20
        spacing: 18

        Image {
            source: "qrc:/icon.png"
            width: 60
            height: 60
            fillMode: Image.PreserveAspectFit
            smooth: true
            visible: parent.width > 500
        }

        ColumnLayout {
            spacing: 4
            Layout.fillWidth: true

    Text {
        text: title
                font.pointSize: fs(32)
        font.bold: true
                color: titleColor
                wrapMode: Text.WordWrap
            }

            Text {
                text: "Monitor Syncthing service & folders"
                font.pointSize: fs(18)
                color: "#1d2844"
                wrapMode: Text.WordWrap
            }
        }

        Rectangle {
            id: closeButton
            width: 64
            height: 64
            radius: 32
            color: accentColor
            border.width: 0
            opacity: 1

            Text {
                anchors.centerIn: parent
                text: "\u00D7"
                font.pointSize: fs(38)
                font.bold: true
                color: "#ffffff"
            }

            MouseArea {
                anchors.fill: parent
                onClicked: closeRequested()
            }
        }
    }
}

