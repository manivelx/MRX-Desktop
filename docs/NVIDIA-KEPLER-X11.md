# NVIDIA Kepler and X11 compatibility notes

MRX targets X11 and does not install, remove, patch, or reconfigure a GPU
kernel module. This is intentional: graphics-driver changes can make a system
unbootable and are outside the desktop installer’s scope.

## Reference hardware profile

The primary development profile supplied for this project is:

- GPU: NVIDIA GeForce GT 740, GK107 / Kepler, 4 GB DDR3
- Driver: proprietary NVIDIA 470.256.02 legacy branch
- Kernel family: CachyOS LTS, reported as Linux 6.18.x
- Display: 1920x1080 over HDMI
- CPU: AMD Ryzen 7 8700F

The Rust WM communicates with the X server through X11/XCB. The QML shell uses
Qt's XCB platform plugin (`QT_QPA_PLATFORM=xcb`). There are no TCP ports to open:
MRX's WM-to-shell control API uses a per-user Unix-domain socket under
`$XDG_RUNTIME_DIR/mrx.sock` (or a private `/tmp/mrx-$USER` fallback).

## Driver and kernel caveats

NVIDIA 470 is the final proprietary branch supporting Kepler and is now a legacy,
unsupported branch. Arch's NVIDIA documentation warns that legacy 470 may not
work with current Xorg versions. See the [ArchWiki NVIDIA page](https://wiki.archlinux.org/title/NVIDIA).

A CachyOS report describes `nvidia-470xx-dkms` failing to load on a 6.18.28
CachyOS LTS kernel because `CONFIG_DRM_AUDIO_COMPONENT` / the corresponding
exported symbol was absent. The report's workaround was to boot standard Arch
`linux-lts`, not to change the desktop. This is a kernel/driver integration
issue, not an MRX issue. See [CachyOS issue #444](https://github.com/CachyOS/distribution/issues/444).

Therefore MRX must not assume that `nvidia-smi` succeeding on another
machine proves the target PC's driver works. Before graphics testing, capture:

```bash
uname -r
lspci -nnk | grep -A3 -E 'VGA|3D|Display'
modinfo nvidia | head
cat /proc/driver/nvidia/version 2>/dev/null || true
nvidia-smi
lsmod | grep -E '^(nvidia|nouveau)'
grep -E 'CONFIG_DRM_AUDIO_COMPONENT|CONFIG_DRM' \
  "/usr/lib/modules/$(uname -r)/build/.config" 2>/dev/null || true
journalctl -k -b | grep -iE 'nvidia|nouveau|drm|unknown symbol' | tail -100
```

Do not automatically install a patched driver or change kernel command-line
options as part of MRX installation. Driver patching must be a separate,
explicit maintenance operation with a known-good boot entry.

## Graphics validation layers

1. **Xvfb:** validates X11 protocol, WM ownership, IPC and basic workspace state.
   It is a software virtual display and cannot validate the NVIDIA driver.
2. **Xephyr:** validates window behavior inside a nested X server on the actual
   desktop. It still shares the host X server and graphics stack.
3. **Real Xorg session:** validates the actual NVIDIA Xorg driver, GLX vendor,
   compositing and frame pacing on the target GPU.

Capture the following on the real PC:

```bash
xdpyinfo | grep -E 'Composite|DAMAGE|RANDR|XFIXES|GLX|Present'
glxinfo -B
nvidia-smi
```

The Xvfb and Xephyr tests must be reported separately from real-GPU tests.
MRX's workspace-slide renderer is not considered hardware-validated until
frame-time measurements are collected on the GT 740 with the 470 driver.
