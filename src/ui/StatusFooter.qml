import QtQuick 2.5
import QtQuick.Layouts 1.3

Rectangle {
    property string statusMessage: ""
    property bool controlBusy: false
    property real fontScale: 1.0

    Layout.fillWidth: true
    height: 60
    radius: 6
    border.width: 1
    border.color: "black"
    color: "#f4f4f4"

    function fs(value) {
        return value * fontScale
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12

        Text {
            text: statusMessage
            font.pointSize: fs(14)
            Layout.fillWidth: true
        }

        Text {
            text: controlBusy ? "Working..." : ""
            font.pointSize: fs(14)
        }
    }
}

