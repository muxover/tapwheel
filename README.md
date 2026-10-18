# TapWheel

<div align="center">

[![CI](https://github.com/muxover/tapwheel/actions/workflows/ci.yml/badge.svg)](https://github.com/muxover/tapwheel/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/muxover/tapwheel)](https://github.com/muxover/tapwheel/releases/latest)

**Drag a file, tap a key, drop it on the wheel.**

</div>

---

TapWheel converts files on Windows 10 and 11 without opening an app. Start dragging a file in File Explorer or on the desktop, tap Shift, and a wheel of formats opens under your pointer. Drop the file on PNG, MP3 or ZIP and the new file appears next to the old one. Everything runs on your PC, and nothing is uploaded. It's inspired by [Tangerine](https://tangerineformac.com/) for Mac and built natively for Windows.

---

## Features

- Three wheels on three keys: Shift converts, Ctrl opens tools, Alt opens actions.
- Converts photos, audio, video, subtitles and archives (see [Formats](#formats)).
- HEIC photos from a phone become JPG or PNG in one move.
- Tap the same key twice to repeat your last choice.
- The formats you pick most move to the top of the wheel.
- Cards in the corner show the size before and after, with Open, Show in folder and Undo.
- Originals are never changed, and a new file never replaces an existing one.
- Follows light or dark mode, your accent colour, high contrast, and the Windows 10 or 11 look.
- Works offline. Nothing is uploaded, and there's no account.

---

## Formats

| Wheel | Files | What you get |
|-------|-------|--------------|
| Convert | Images | JPG, PNG, WebP, AVIF, GIF, TIFF, BMP, ICO (HEIC is read) |
| Convert | Audio | MP3, M4A, WAV, FLAC, OGG, Opus |
| Convert | Video | MP4, MOV, MKV, WebM, AVI, GIF, MP3 |
| Convert | Subtitles | SRT, VTT, plain text |
| Convert | Archives, folders | ZIP, 7Z, TAR, TAR.GZ |
| Tools | Images | Compress, half size, fit 1920, rotate, strip location data |
| Tools | Video | Compress, 720p, mute, save the sound |
| Tools | Audio | Compress, even out the volume |
| Tools | Archives | Extract |
| Tools | Anything | Pack into one ZIP |
| Actions | Anything | Copy path, copy files, open a terminal, show in File Explorer |

---

## Installation

- [Microsoft Store](https://apps.microsoft.com/detail/9MW1GB4106VS): signed by Microsoft, updates itself.
- [Portable zip](https://github.com/muxover/tapwheel/releases/latest): unzip anywhere and run `tapwheel.exe`. No admin rights needed, and it adds a right-click entry.

The zip isn't code-signed, so SmartScreen may ask once before the first run.

---

## Quick Start

1. Run TapWheel. Its window has a button that makes sample files to practise on.
2. Start dragging a file and keep the mouse button down.
3. Tap Shift. The Convert wheel opens under the pointer.
4. Drop on a slice. Drop in the middle, or press Esc, to cancel.

Tap Ctrl or Alt on an open wheel to switch to Tools or Actions. Closing the window keeps TapWheel in the tray.

---

## Configuration

Settings live in the app's window and are saved to `%APPDATA%\TapWheel\settings.toml`.

| Setting | Default | What it does |
|---------|---------|--------------|
| Convert key | Shift | Key to tap during a drag for the Convert wheel |
| Tools key | Ctrl | Key for the Tools wheel |
| Actions key | Alt | Key for the Actions wheel |
| Start with Windows | On | Starts TapWheel in the tray when you sign in |
| Favorites first | On | Moves the slices you pick most to the top |
| Pause TapWheel | Off | Ignores key taps until turned off again or TapWheel restarts |

---

## Privacy

- No network connections: no account, no telemetry, no update checks.
- Files are converted on your PC by a bundled FFmpeg and stay in their folder.
- The keyboard and mouse are watched only to spot a quick tap of Shift, Ctrl or Alt during a drag.
- Nothing you type is recorded, and every key and click passes through untouched.

---

## Limitations

- Drags from apps running as administrator can't reach the wheel.
- Files Explorer shows inside a ZIP or on a phone need copying to a folder first.
- No documents, PDFs or Office files yet.
- AVIF and WebM are slow on large files.

---

## Project Layout

```text
tapwheel/
├── crates/
│   ├── core/                 # Formats, output names, the converter runner, archives, subtitles, settings
│   ├── ui/                   # Draws the wheel, progress cards, home window and icon
│   └── app/                  # The Windows app: key and mouse watching, wheel window, drops, tray
├── engines/                  # Pinned FFmpeg build and the script that fetches it
├── packaging/                # Portable zip and Store package builder, Store manifest
├── site/                     # tapwheel.muxover.com and its layout check
├── release-notes/            # One file per release
└── docs/PROJECT.md           # Maintainer notes
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

Licensed under the [GPL-3.0](LICENSE) license. Third-party parts are listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

---

## Links

- Website: https://tapwheel.muxover.com
- Microsoft Store: https://apps.microsoft.com/detail/9MW1GB4106VS
- Repository: https://github.com/muxover/tapwheel
- Issues: https://github.com/muxover/tapwheel/issues
- Changelog: [CHANGELOG.md](CHANGELOG.md)

---

<p align="center">Made with ❤️ by Jax (@muxover)</p>
