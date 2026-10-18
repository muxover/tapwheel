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

Get it from the [Microsoft Store](https://apps.microsoft.com/detail/9MW1GB4106VS). The Store version is signed by Microsoft and updates itself.

Or download `TapWheel-<version>-windows-x64.zip` from the [latest release](https://github.com/muxover/tapwheel/releases/latest), unzip it anywhere and run `tapwheel.exe`. It installs nothing else and needs no admin rights. The portable build also adds TapWheel to the right-click menu ("Show more options" on Windows 11).

The zip isn't code-signed, so Windows SmartScreen may ask before the first run.

---

## Quick Start

1. Run TapWheel. Its window shows the keys and a button that makes a folder of sample files to practise on.
2. Start dragging any file. Keep the mouse button down.
3. Tap Shift. The Convert wheel opens under the pointer.
4. Drop the file on a slice. Drop it in the middle, or press Esc, to cancel.

Tap Ctrl or Alt while the wheel is open to switch to Tools or Actions. Closing the window leaves TapWheel running in the tray.

---

## Configuration

Settings live in the app's window and are saved to `%APPDATA%\TapWheel\settings.toml`.

| Setting | Default | What it does |
|---------|---------|--------------|
| Convert key | Shift | Key you tap during a drag for the Convert wheel |
| Tools key | Ctrl | Key for the Tools wheel |
| Actions key | Alt | Key for the Actions wheel |
| Start with Windows | On | Starts TapWheel in the tray when you sign in |
| Favorites first | On | Moves the slices you pick most to the top of each wheel |
| Pause TapWheel | Off | Ignores key taps until you turn it back off (resets when TapWheel restarts) |

---

## Privacy

TapWheel makes no network connections. There's no account, no telemetry and no update check. Files are converted by a copy of FFmpeg that ships inside the app, and the results stay in the folder you dragged from. To see when you tap a key during a drag, TapWheel watches the keyboard and mouse while it runs. It only acts on Shift, Ctrl, Alt and Esc, never records what you type, and passes every key and click on untouched.

---

## Limitations

- Windows doesn't let a normal app see drags that start in an app running as administrator, so the wheel won't open over those.
- Files inside a ZIP or on a phone that Explorer shows without copying them first can't be converted. Copy them to a folder first.
- Documents, PDFs and Office files aren't supported yet.
- AVIF and WebM take noticeably longer than other formats on large inputs.

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
