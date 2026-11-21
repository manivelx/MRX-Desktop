import QtQuick
import Quickshell
import QtQuick.Layouts
import qs.services
import qs.modules.common
import qs.modules.common.widgets

ContentPage {
    forceWidth: true
    bottomContentPadding: 32
    readonly property var settings: WM.settings ?? ({})
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 16
        StyledText { Layout.fillWidth: true; text: "MRX desktop"; font.pixelSize: Appearance.font.pixelSize.large; color: Appearance.colors.colOnSurface }
        StyledText { Layout.fillWidth: true; wrapMode: Text.Wrap; text: "These controls update the running Rust window manager. Settings that are not implemented yet are intentionally omitted rather than shown as fake toggles."; color: Appearance.colors.colSubtext }
        ConfigSlider { Layout.fillWidth: true; text: "Window gap"; from: 0; to: 40; stepSize: 1; value: settings.appearance?.gaps ?? 8; onMoved: WM.setConfig("window_gap", Math.round(value)) }
        ConfigSlider { Layout.fillWidth: true; text: "Border width"; from: 0; to: 8; stepSize: 1; value: settings.appearance?.border_width ?? 2; onMoved: WM.setConfig("border_width", Math.round(value)) }
        ConfigSlider { Layout.fillWidth: true; text: "Master window ratio"; from: 0.3; to: 0.75; stepSize: 0.01; value: settings.window?.master_ratio ?? 0.58; onMoved: Quickshell.execDetached(["mrxctl", "set", "window.master_ratio", String(value)]) }
        ConfigComboBox { Layout.fillWidth: true; text: "Layout"; model: [{displayName:"Master and stack",value:"master-stack"},{displayName:"Monocle",value:"monocle"}]; currentValue: settings.window?.layout ?? "master-stack"; onSelected: value => WM.setConfig("layout", value) }
        Item { Layout.fillWidth: true; implicitHeight: 8 }
        StyledText { Layout.fillWidth: true; text: "Current support boundaries"; font.pixelSize: Appearance.font.pixelSize.medium; font.weight: 600; color: Appearance.colors.colOnSurface }
        StyledText {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Appearance.colors.colSubtext
            text: "Available: native window layout, workspace switching, monitor-aware window movement, gaps and borders. Not yet available: workspace animations/compositor effects, global shell shortcuts managed by MRX, video wallpapers, and MRX-owned night-light/gamma control. Hyprland-only shader and compositor controls are hidden in this session."
        }
        NoticeBox {
            Layout.fillWidth: true
            text: "MRX is experimental. Unsupported controls are intentionally disabled or omitted rather than invoking Hyprland/BSPWM commands."
        }
    }
}
