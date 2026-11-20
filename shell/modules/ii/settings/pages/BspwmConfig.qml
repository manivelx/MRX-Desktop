import QtQuick
import Quickshell
import QtQuick.Layouts
import qs.services
import qs.modules.common
import qs.modules.common.widgets

ContentPage {
    forceWidth: true
    bottomContentPadding: 32

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 16
        StyledText {
            Layout.fillWidth: true
            text: "MRX desktop controls"
            font.pixelSize: Appearance.font.pixelSize.large
            color: Appearance.colors.colOnSurface
        }
        StyledText {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: "These settings are applied live by the MRX Rust window manager. They do not modify your other desktop sessions."
            color: Appearance.colors.colSubtext
        }
        ConfigSlider {
            Layout.fillWidth: true
            text: "Window gap"
            from: 0; to: 40; stepSize: 1
            value: 8
            onMoved: WM.setConfig("window_gap", Math.round(value))
        }
        ConfigSlider {
            Layout.fillWidth: true
            text: "Border width"
            from: 0; to: 8; stepSize: 1
            value: 2
            onMoved: WM.setConfig("border_width", Math.round(value))
        }
        ConfigComboBox {
            Layout.fillWidth: true
            text: "Default layout"
            model: [{displayName: "Master and stack", value: "master-stack"}, {displayName: "Monocle", value: "monocle"}]
            currentValue: "master-stack"
            onSelected: value => WM.setConfig("layout", value)
        }
        ConfigSwitch {
            Layout.fillWidth: true
            text: "Focus follows pointer"
            checked: false
            onCheckedChanged: if (checked !== undefined) WM.setConfig("focus_follows_pointer", checked ? "true" : "false")
        }
    }
}
