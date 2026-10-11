# DitherCraft app icon

**Creature:** a mechanical/engraved Rust Crab ("Ferris" meets retro engraving, DSP dithering, and circuit aesthetics), DitherCraft's studio mascot.

**Style:** an engraving-style portrait in the studio workstation template: a detailed mechanical carapace and claw portrait on a full-bleed deep slate blue field, with the claw and walking legs gracefully running off the tile edge.

**Palette** (exactly three colours):

| Colour | Hex | Used for |
|---|---|---|
| Ink | `#0b0b0c` | line work, circuit engraving, and contour |
| Paper | `#ede7db` | mechanical plates, carapace, and claw body |
| DitherCraft Deep Slate | `#1e3d4f` | the full-bleed backdrop field |

**Tile:** `viewBox="0 0 512 512"`, a rounded squircle with `rx=112` (continuous curvature), clipped with transparent outer corners. The macOS files add Apple's transparent margin (tile = 824/1024 of the canvas); Windows and Linux use the tile edge to edge.

**Provenance:** high-resolution master artwork rendered at 2048 px, downsampled with Lanczos antialiasing to standard multi-resolution assets across `.png`, `.ico`, `.icns`, and `.svg`. Licence: see `LICENSE.txt`.

## Files

| File | What | Used by |
|---|---|---|
| `dithercraft.svg` | master vector (traced at 2048 px, about 700 KB) | source for everything below |
| `dithercraft-small.svg` | lighter vector (traced at 1024 px, about 370 KB) | `hicolor/scalable` |
| `dithercraft-1024.png` | full tile, 1024 px | docs, store listings |
| `dithercraft-macos-512.png` | tile with Apple margin, 512 px | runtime Dock icon (`apps/dithercraft/src/main.rs`) |
| `dithercraft.icns` | macOS icon set, 16 to 1024 px | `cargo xtask bundle` (`CFBundleIconFile`) |
| `dithercraft.ico` | Windows icon, 16 to 256 px | `apps/dithercraft/build.rs` (embedded in the `.exe`) |
| `hicolor/<size>/apps/ai.storyteller.dithercraft.png` | Linux theme icons, 16 to 512 px; the 256 one is also the runtime icon on Windows and Linux, the 128 one the brand mark in the app bar and About box (`crates/ui-egui/src/brand.rs`) | `packaging/linux/ai.storyteller.dithercraft.desktop` |
| `hicolor/scalable/apps/ai.storyteller.dithercraft.svg` | Linux scalable icon | as above |

## How it reaches each OS

- **macOS:** the app sets the Dock / app-switcher icon at runtime (`ViewportBuilder::with_icon`); the
  `.app` from `cargo xtask bundle` carries `dithercraft.icns`.
- **Windows:** `build.rs` embeds `dithercraft.ico` and VERSIONINFO with `winresource` (taskbar, Start
  menu, Explorer, Alt-Tab); the runtime icon covers the title bar.
- **Linux:** install `packaging/linux/ai.storyteller.dithercraft.desktop` to `share/applications/` and
  `hicolor/` to `share/icons/`. The app sets its Wayland app ID / X11 class to
  `ai.storyteller.dithercraft`, so the dock matches the window to the launcher.

## Regenerate

Edit or replace `dithercraft.svg` (and `dithercraft-small.svg`), then run `packaging/icons.sh`
(needs `resvg`; `iconutil` on macOS for the `.icns`). It writes every PNG, the `.ico` (via
`cargo xtask ico`) and the `.icns`.
