# XComposite and XDamage prototype

## Scope

\`mrx-graphics-probe\` is an isolated protocol prototype. It connects to the current X11 display, negotiates XComposite and XDamage, creates one disposable override-redirect test window, manually redirects only that window, names its off-screen pixmap, and verifies the pixmap geometry. It then creates an XDamage object, paints a deterministic region and requires a non-empty damage notification before cleaning up all resources.

The full probe runs from the Xvfb integration suite and again inside nested Xephyr, so CI validates the protocol path against two software X servers.

## Safety boundary

The production MRX WM does **not** redirect managed client windows. Manual Composite redirection changes how window contents reach the screen and must not be enabled while an unrelated compositor such as Picom owns the compositor role. The disposable probe runs before the WM starts and redirects only its own temporary window.

This prototype does not provide a renderer, texture upload, GLX context, Present scheduling, frame pacing, workspace animation or a user-visible compositor. The desktop continues to use its normal X11 rendering path. Do not mark graphics support or GPU acceleration as available based on this probe.

## Run

\`\`\`bash
cargo build --workspace
bash ./tests/xvfb-smoke.sh
bash ./tests/xephyr-smoke.sh
\`\`\`

Expected output includes successful XComposite named-pixmap capture and XDamage event delivery. Real GLX/Present and GPU tests remain reserved for a physical CachyOS-LTS test session.
