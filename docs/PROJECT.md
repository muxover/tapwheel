# TapWheel — maintainer notes

## What this is

TapWheel is a free, open-source Windows 10 and 11 app for converting files without opening anything. You start dragging a file, tap Shift, Ctrl or Alt, and a wheel opens under the pointer; dropping on a slice converts, compresses or acts on the file, and the result lands next to the original. It's inspired by Tangerine for Mac and built natively for Windows.

## Stack and layout

- Rust workspace, GPL-3.0-or-later.
- `crates/core`: the format matrix, output naming, the ffmpeg runner, archives, subtitles, PNG compression and settings. Plain Rust, tested on Linux and Windows.
- `crates/ui`: draws the wheel, progress cards, home window and icon with tiny-skia and ab_glyph. No Win32, so every surface renders to PNG in tests (`cargo run -p tapwheel-ui --example preview`).
- `crates/app`: the Windows app on windows-sys: low-level keyboard and mouse hooks, a layered wheel window registered as an OLE drop target, the tray icon, progress cards and the home window.
- `engines/`: pinned FFmpeg build (version, URL, SHA-256) and `fetch.ps1`. Never committed.
- `packaging/`: `package.ps1` builds the portable zip and the Store MSIX; `AppxManifest.xml` holds the Store identity.

## Decisions

- 2026-10-18: Rust and Win32 directly, no Electron or WebView, so the app stays small and the wheel looks native on both Windows 10 and 11.
- 2026-10-18: GPL-3.0-or-later with FFmpeg bundled, so it works offline right after install.
- 2026-10-18: the wheel opens on a tap (down and up within 400 ms, no other key between), never a hold, and the drop target only ever answers COPY or NONE. Explorer reads MOVE as "delete the source", so MOVE is never returned.
- 2026-10-18: outputs go next to the source, written to `name.ext.tw-part` and renamed at the end; an existing file is never replaced.
- 2026-10-18: the UI follows the system: light or dark, the accent colour, high contrast, Segoe UI Variable and rounded corners on Windows 11, Segoe UI and square corners on Windows 10.
- 2026-10-18: Store identity `Jaxmuxover.TapWheel`, publisher `CN=50CBFD77-B27F-4D02-AB74-844DC5756318`, display name "Jax (muxover)".
- 2026-10-18: the Store package has no right-click entry (MSIX can't write the shell keys Explorer reads); the portable build adds a classic one.

## Status

v0.1.0 in progress.

## Next

- First green CI run on Windows, then hands-on testing on a real PC.

## Open questions

- None.
