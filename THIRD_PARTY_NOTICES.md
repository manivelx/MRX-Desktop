# Third-party notices

MRX incorporates or references software and assets maintained by other projects. Their original license and copyright notices must be retained.

## end4-pC / Quickshell shell source

The `shell/` subtree is distributed under the retained GNU General Public License version 3 (`GPL-3.0`) from the supplied end4-pC X11 compatibility port. The full upstream license is retained in `shell/LICENSE` and copied to `docs/END4-LICENSE`. Modified or bundled copies of that subtree must preserve the applicable GPL-3.0 terms; the root MIT license does not relicense upstream-derived shell code. MRX does not claim authorship of upstream files. Treat new files placed inside `shell/` as part of this GPL-3.0 subtree unless a separate license review establishes otherwise. The root `LICENSE` applies to original MRX-authored code outside that subtree, subject to the licenses of dependencies and other third-party materials.

## Rust dependencies

Rust dependencies are resolved through Cargo manifests. Generate and review a dependency/license inventory before each binary release. Do not assume that the repository's MIT license replaces dependency licenses.

## Other assets

Before adding fonts, icons, wallpapers, screenshots, translations or other media, record their source and license here or in an adjacent notice file. Remove assets whose redistribution rights cannot be established.
