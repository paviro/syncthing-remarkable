import QtQuick 2.5
import QtQuick.Layouts 1.3

RowLayout {
    property string title: "Syncthing"
    property string lastUpdated: ""
    property real fontScale: 1.0

    Layout.fillWidth: true
    spacing: 12

    function fs(value) {
        return value * fontScale
    }

    Text {
        text: title
        font.pointSize: fs(34)
        font.bold: true
    }

    Item {
        Layout.fillWidth: true
    }

    Text {
        text: lastUpdated ? "Updated " + lastUpdated : "Waiting for data..."
        font.pointSize: fs(14)
    }
}

