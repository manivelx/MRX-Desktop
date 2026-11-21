# Phase 3 — Graphics and rendering status

## Implemented and CI-covered

- A standalone graphics probe manually redirects only a disposable override-redirect test window with XComposite.
- The probe names and validates the captured pixmap dimensions.
- XDamage reports a deterministic non-empty update and the test acknowledges/cleans up the damage object.
- The probe creates XRender pictures for the captured pixmap and an off-screen destination, clears the destination, composites with `PictOp::OVER`, reads the destination pixels back and rejects an empty render.
- Ubuntu and native Arch Linux CI run the probe under Xvfb and nested Xephyr. Latest evidence: [PR #27 run #672](https://github.com/manivelx/MRX-Desktop/actions/runs/44920294014).

## Not implemented — production renderer

- No compositor-manager selection (`_NET_WM_CM_Sn`) ownership.
- No root `Subwindows` redirection or capture of managed application windows.
- No Composite overlay window painting/presentation or input-shape handling.
- No live per-window pixmap/damage lifecycle, resize/hide/destroy recovery or occlusion handling.
- No GLX/Present accelerated presentation path, frame pacing, vsync or workspace transitions.
- No validated no-animation production fallback integrated with the session manager.

## Hardware and release gates

- Validate XRender/GLX behavior on a booted CachyOS-LTS system.
- Test the NVIDIA GT 740 with the proprietary 470xx driver and confirm a stable fallback.
- Test multi-monitor hotplug, mixed resolutions, fullscreen, transparency and long-running sessions.
- Keep production root redirection disabled until the renderer can acquire/release ownership and recover cleanly after crashes.

The current work is a validated off-screen rendering building block, not a production compositor.