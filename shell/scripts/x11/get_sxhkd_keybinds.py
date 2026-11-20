#!/usr/bin/env python3
import argparse, json, os, re

p = argparse.ArgumentParser()
p.add_argument('--path', required=True)
a = p.parse_args()
path = os.path.expandvars(os.path.expanduser(a.path))
try:
    lines = open(path, encoding='utf-8').read().splitlines()
except OSError:
    print(json.dumps({'name': 'BSPWM', 'children': [{'name': 'BSPWM', 'children': [], 'keybinds': []}], 'keybinds': []}))
    raise SystemExit(0)

items = []
pending = ''
i = 0
while i < len(lines):
    raw = lines[i]
    line = raw.strip()
    if not line:
        i += 1; continue
    if line.startswith('#'):
        comment = line.lstrip('#').strip()
        if comment and comment.lower() not in {'apps', 'workspace', 'windows', 'session', 'screenshots', 'system'} and not comment.startswith('end4-pC BSPWM'):
            pending = comment
        i += 1; continue
    if raw[:1].isspace():
        i += 1; continue
    key = line
    command = ''
    j = i + 1
    while j < len(lines):
        if not lines[j].strip():
            j += 1; continue
        if lines[j][:1].isspace():
            command = lines[j].strip(); break
        break
    if command:
        parts = [x.strip() for x in key.split('+')]
        mods = []
        keyname = parts[-1] if parts else key
        for m in parts[:-1]:
            mm = m.upper()
            if mm in {'SUPER','SHIFT','CTRL','ALT','META'}: mods.append(mm)
        comment = pending
        if not comment:
            c = command.lower()
            if 'toggleSettings'.lower() in c: comment = 'Open settings'
            elif 'toggleSidebarLeft'.lower() in c: comment = 'Toggle left sidebar'
            elif 'toggleSidebarRight'.lower() in c: comment = 'Toggle control center'
            elif 'toggleOverview'.lower() in c or 'search toggle' in c: comment = 'Open overview and launcher'
            elif 'toggleWallpaperSelector'.lower() in c: comment = 'Open wallpaper selector'
            elif 'toggleMediaControls'.lower() in c: comment = 'Toggle media controls'
            elif 'toggleSession'.lower() in c: comment = 'Open session menu'
            elif 'lock-session' in c: comment = 'Lock screen'
            elif 'desktop -f' in c: comment = 'Switch workspace'
            elif 'node -d' in c: comment = 'Move window to workspace'
            elif 'node -c' in c: comment = 'Close focused window'
            elif 'node -t fullscreen' in c: comment = 'Toggle fullscreen'
            elif 'node -t floating' in c: comment = 'Toggle floating window'
            elif 'node -f' in c: comment = 'Focus window in direction'
            elif 'node -s' in c: comment = 'Swap window in direction'
            elif 'wm -r' in c: comment = 'Restart BSPWM'
            elif 'ipc call region screenshot' in c: comment = 'Take region screenshot'
            elif 'kitty' in c: comment = 'Open terminal'
            else: comment = command[:64]
        items.append({'mods': mods, 'key': keyname, 'dispatcher': 'sxhkd', 'params': command, 'comment': comment})
        pending = ''
        i = j + 1
    else:
        i += 1
print(json.dumps({'name': 'BSPWM', 'children': [{'name': 'BSPWM', 'children': [], 'keybinds': items}], 'keybinds': []}, ensure_ascii=False))
