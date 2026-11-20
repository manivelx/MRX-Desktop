import QtQuick
import Quickshell.Io
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions

Item {
    id: root
    required property var windowData
    property string previewPath: `${FileUtils.trimFileProtocol(Directories.cache)}/overview/${String(windowData.id).replace(/[^a-zA-Z0-9_-]/g, "_")}.png`
    implicitWidth: 148
    implicitHeight: 82

    Process {
        id: captureProc
        command: ["bash", "-c", `mkdir -p '${StringUtils.shellSingleQuoteEscape(FileUtils.trimFileProtocol(Directories.cache) + "/overview")}' && maim -i '${StringUtils.shellSingleQuoteEscape(String(root.windowData?.xid ?? ""))}' '${StringUtils.shellSingleQuoteEscape(root.previewPath)}'`]
        onExited: {
            preview.source = ""
            preview.source = `file://${root.previewPath}`
        }
    }
    Timer {
        interval: 1800
        repeat: true
        running: root.visible && !!root.windowData?.xid
        onTriggered: if (!captureProc.running) captureProc.running = true
    }

    Rectangle {
        anchors.fill: parent
        radius: Appearance.rounding.small
        color: Appearance.colors.colLayer2
        clip: true
        Image {
            id: preview
            anchors.fill: parent
            fillMode: Image.PreserveAspectFit
            cache: false
            asynchronous: true
        }
        StyledText {
            anchors.centerIn: parent
            visible: !preview.source || preview.status !== Image.Ready
            text: root.windowData.appId || "Window"
            color: Appearance.colors.colSubtext
            elide: Text.ElideRight
        }
    }
}
