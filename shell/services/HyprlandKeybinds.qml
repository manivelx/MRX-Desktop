pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import qs.services

/**
 * A service that provides access to Hyprland keybinds.
 * Uses the `get_keybinds.py` script to parse comments in config files in a certain format and convert to JSON.
 */
Singleton {
    id: root
    property string keybindParserPath: FileUtils.trimFileProtocol(`${Directories.scriptPath}/hyprland/get_keybinds.py`)
    property string defaultKeybindConfigPath: FileUtils.trimFileProtocol(`${Directories.config}/hypr/hyprland/keybinds.lua`)
    property string userKeybindConfigPath: FileUtils.trimFileProtocol(`${Directories.config}/hypr/custom`)
    property string bspwmKeybindParserPath: FileUtils.trimFileProtocol(`${Directories.scriptPath}/x11/get_sxhkd_keybinds.py`)
    property string bspwmKeybindConfigPath: FileUtils.trimFileProtocol(`${Directories.config}/mrx/sxhkdrc`)
    property bool isMrx: WM.compositor === "mrx"
    property var defaultKeybinds: {"children": []}
    property var userKeybinds: {"children": []}
    property var keybinds: ({
        children: [
            ...(defaultKeybinds.children ?? []),
            ...(userKeybinds.children ?? []),
        ]
    })

    Connections {
        target: Hyprland
        enabled: WM.compositor === "hyprland"

        function onRawEvent(event) {
            if (event.name == "configreloaded") {
                getDefaultKeybinds.running = true
                getUserKeybinds.running = true
            }
        }
    }

    Process {
        id: getDefaultKeybinds
        running: WM.compositor === "hyprland"
        command: [root.keybindParserPath, "--path", root.defaultKeybindConfigPath]
        
        stdout: SplitParser {
            onRead: data => {
                try {
                    root.defaultKeybinds = JSON.parse(data)
                } catch (e) {
                    console.error("[CheatsheetKeybinds] Error parsing keybinds:", e)
                }
            }
        }
    }

    Process {
        id: getBspwmKeybinds
        running: WM.compositor === "bspwm" || root.isMrx
        command: ["python3", root.bspwmKeybindParserPath, "--path", root.bspwmKeybindConfigPath]
        stdout: StdioCollector {
            onStreamFinished: {
                try { root.defaultKeybinds = JSON.parse(text) }
                catch (e) { console.error("[BSPWMKeybinds] Error parsing sxhkdrc:", e) }
            }
        }
    }

    Process {
        id: getUserKeybinds
        running: WM.compositor === "hyprland"
        command: [root.keybindParserPath, "--path", root.userKeybindConfigPath]
        
        stdout: SplitParser {
            onRead: data => {
                try {
                    root.userKeybinds = JSON.parse(data)
                } catch (e) {
                    console.error("[CheatsheetKeybinds] Error parsing keybinds:", e)
                }
            }
        }
    }
}

