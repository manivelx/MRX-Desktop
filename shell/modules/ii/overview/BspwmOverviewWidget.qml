import QtQuick
import QtQuick.Layouts
import qs.services
import qs.modules.common
import qs.modules.common.widgets

Item {
    id: root
    required property var screen
    readonly property var monitor: WM.monitorFor(screen)
    readonly property var workspaces: WM.workspaces.filter(w => w.output === monitor?.name)
    implicitWidth: Math.min((screen?.width ?? 1200) * 0.88, 1080)
    implicitHeight: Math.min((screen?.height ?? 800) * 0.68, 560)

    Rectangle {
        anchors.fill: parent
        radius: Appearance.rounding.large
        color: Appearance.colors.colLayer1
        border.width: 1
        border.color: Appearance.colors.colLayer2

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 20
            spacing: 14
            StyledText {
                Layout.fillWidth: true
                text: "Workspaces"
                font.pixelSize: Appearance.font.pixelSize.large
                color: Appearance.colors.colOnSurface
            }
            GridLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                columns: 3
                rowSpacing: 12
                columnSpacing: 12
                Repeater {
                    model: root.workspaces
                    delegate: Rectangle {
                        id: workspaceCard
                        required property var modelData
                        property var workspaceWindows: WM.windowList.filter(w => w.workspaceId === modelData.id && w.monitorName === root.monitor?.name)
                        property var largestWindow: workspaceWindows.length > 0
                            ? workspaceWindows.reduce((a, b) => a.width * a.height >= b.width * b.height ? a : b)
                            : null
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.minimumHeight: 112
                        radius: Appearance.rounding.normal
                        color: modelData.is_active ? Appearance.colors.colPrimaryContainer : Appearance.colors.colLayer2
                        border.width: modelData.is_active ? 2 : 1
                        border.color: modelData.is_active ? Appearance.colors.colPrimary : Appearance.colors.colLayer3
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 8
                            RowLayout {
                                Layout.fillWidth: true
                                StyledText {
                                    text: "Workspace " + workspaceCard.modelData.name
                                    color: workspaceCard.modelData.is_active ? Appearance.colors.colOnPrimaryContainer : Appearance.colors.colOnLayer1
                                    font.weight: Font.DemiBold
                                }
                                Item { Layout.fillWidth: true }
                                StyledText {
                                    text: String(workspaceCard.modelData.windows)
                                    color: Appearance.colors.colSubtext
                                }
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                X11WindowPreview {
                                    visible: !!workspaceCard.largestWindow
                                    windowData: workspaceCard.largestWindow ?? ({})
                                    Layout.preferredWidth: 116
                                    Layout.preferredHeight: 66
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    StyledText {
                                        Layout.fillWidth: true
                                        text: workspaceCard.largestWindow?.appId ?? "Empty workspace"
                                        color: workspaceCard.modelData.is_active ? Appearance.colors.colOnPrimaryContainer : Appearance.colors.colOnLayer1
                                        elide: Text.ElideRight
                                    }
                                    StyledText {
                                        Layout.fillWidth: true
                                        text: workspaceCard.largestWindow?.title ?? ""
                                        color: Appearance.colors.colSubtext
                                        elide: Text.ElideRight
                                    }
                                }
                            }
                            Item { Layout.fillHeight: true }
                        }
                        MouseArea {
                            anchors.fill: parent
                            z: -1
                            onClicked: {
                                WM.switchWorkspaceOnMonitor(workspaceCard.modelData.id, root.monitor?.name ?? "")
                                GlobalStates.overviewOpen = false
                            }
                        }
                    }
                }
            }
        }
    }
}
