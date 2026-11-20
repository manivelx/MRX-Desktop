import QtQuick
import Quickshell
import Quickshell.Io
import qs.modules.common
import qs.modules.common.functions
import qs.services

Process {
    id: screenshotProc
    running: true
    property string screenshotDir: Directories.screenshotTemp
    required property ShellScreen screen
    property string screenshotPath: `${screenshotDir}/image-${screen.name}.png`
    readonly property var geometry: WM.monitorGeometry(screen)
    readonly property string captureCommand: WM.isX11
        ? `maim -g '${screen.width}x${screen.height}+${geometry.x}+${geometry.y}'`
        : `grim -o '${StringUtils.shellSingleQuoteEscape(screen.name)}'`
    command: ["bash", "-c", `mkdir -p '${StringUtils.shellSingleQuoteEscape(screenshotDir)}' && ${captureCommand} '${StringUtils.shellSingleQuoteEscape(screenshotPath)}'`]
}
