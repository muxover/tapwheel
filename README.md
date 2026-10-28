# TapWheel

<div align="center">

[![CI](https://github.com/muxover/tapwheel/actions/workflows/ci.yml/badge.svg)](https://github.com/muxover/tapwheel/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/muxover/tapwheel)](https://github.com/muxover/tapwheel/releases/latest)

**Drag a file, tap a key, drop it on the wheel.**

</div>

---

TapWheel puts file tools on a wheel in Windows 10 and 11. Start dragging a file in File Explorer or on the desktop, tap Shift, Ctrl or Alt, and a wheel opens under your pointer. Drop the file on JPG, Trim, Remove BG or ZIP and the new file appears next to the old one. Pictures, video, sound, icons, text, code and archives, all done on your PC, with nothing uploaded. It's inspired by [Tangerine](https://tangerineformac.com/) for Mac and built natively for Windows. Try the tool windows at [tapwheel.app](https://tapwheel.app/#try).

---

## Features

- Three wheels on three keys: Shift converts, Ctrl opens tools, Alt opens actions.
- The wheel only opens while you're dragging files, and shows only what fits them.
- Rest on a slice for its choices on the outer ring, like Balanced or 25 MB.
- Tools that need setting up open their own window with a live preview: crop, backgrounds, adjust, annotate, redact, trim, GIFs, frame snapshots, combine, code shots, photo details, song tags, archive contents, passwords, rename and file info.
- Remove BG runs on your PC with a bundled model. Touch up opens the cut-out with Keep and Remove brushes that can follow edges.
- More than eight slices turn into pages. Rest on More, press Tab, or roll the mouse wheel.
- Tap the same key twice to repeat your last choice; favourites move to the top.
- Compact cards show the size before and after, with Open, Show, Undo, Cancel and Retry. Undo only removes a file TapWheel made, and only if you haven't changed it since.
- A job checks the file can be read and the disk has room before it starts. If TapWheel is closed mid-job, the leftover part-files are cleared next time.
- Tools never change the original, and a new file never replaces an existing one. Only Rename and Move to touch the original, when you pick them.
- Follows light or dark mode, your accent colour, high contrast, and the Windows 10 or 11 look.
- Works offline. Nothing is uploaded, and there's no account.

---

## Formats

| Kind | Opens | Convert to | Tools |
|------|-------|------------|-------|
| Pictures | 48 formats, including HEIC, JPEG XL, PSD and camera RAW | PNG, JPG, WebP, AVIF, GIF, TIFF, BMP, ICO, TGA, QOI | Crop, Add BG, Remove BG, Compress, Resize, Adjust, Annotate, Redact, Rotate, Metadata, Make icon, To SVG, Combine |
| Video | 24 formats | MP4, MOV, MKV, WebM, GIF, MP3, AVI, WMV, M4A, WAV | Trim, Compress, Make GIF, Resize, Crop, Speed, Snapshot, Mute, Get audio, Rotate, Reverse, Join |
| Audio | 31 formats | MP3, M4A, WAV, FLAC, OGG, Opus, AIFF, WMA, M4R | Trim, Fade, Normalize, Compress, Volume, Speed, Metadata, Ringtone, Mono, No silence, Reverse, Join |
| Icons | ICO, ICNS, CUR, SVG | PNG, ICO, ICNS, WebP, JPG | Windows icon, Favicon, PNG set, macOS icon, Minify |
| Text | 42 formats, including CSV, JSON, YAML, subtitles and RTF | Markdown, HTML and data formats | Read aloud, UTF-8, Clean up, Sort, Dedupe, Case, Line ends, Split, Count, Merge, Format, Minify |
| Code | 90 formats | PNG, HTML, Markdown, TXT | Code shot, Copy as MD, Format, Minify, Indent, Line ends, Clean up, UTF-8, Merge |
| Archives | 19 formats | ZIP, 7Z, TAR, TAR.GZ, TAR.XZ, TAR.BZ2, TAR.ZST | Extract, Contents, Test, Password, Split |

Data, config and subtitle files only get the text tools that keep their structure whole. Every file also gets ZIP and the Actions wheel: Copy, Copy path, Rename, Move to, Duplicate, Share, File info, Float, Checksum, Properties and Open with, plus Terminal for folders and code. The full list is in [site/public/data/wheel.json](site/public/data/wheel.json), made from the app.

---

## Installation

- [Microsoft Store](https://apps.microsoft.com/detail/9MW1GB4106VS): the one to get. Signed by Microsoft and updates itself.
- winget, from a terminal: `winget install 9MW1GB4106VS` (the Store version).
- [Portable zip](https://github.com/muxover/tapwheel/releases/latest), for developers and testers: unzip anywhere and run `tapwheel.exe`. No admin rights needed, and it adds a right-click entry. It isn't code-signed, so your browser may flag the download and SmartScreen may ask once before the first run.

---

## Quick Start

1. Run TapWheel. Its window shows how it works and makes sample files to practise on.
2. Start dragging a file and keep the mouse button down.
3. Tap Shift. The Convert wheel opens under the pointer.
4. Drop on a slice, or rest on it and drop on a choice on the ring. Drop in the middle, or press Esc, to cancel.

Tap Ctrl or Alt on an open wheel to switch to Tools or Actions. Closing the window keeps TapWheel in the tray.

---

## Configuration

Settings live in the app's window and are saved to `%APPDATA%\TapWheel\settings.toml`.

| Setting | Default | What it does |
|---------|---------|--------------|
| Convert key | Shift | Key to tap during a drag for the Convert wheel |
| Tools key | Ctrl | Key for the Tools wheel |
| Actions key | Alt | Key for the Actions wheel |
| Without dragging | Double tap | Opens a wheel on the files selected in File Explorer or on the desktop: tap its key twice, press Ctrl+Shift+Space, or Off |
| Start with Windows | On | Starts TapWheel in the tray when you sign in |
| Favorites first | On | Moves the slices you pick most to the top |
| Pause TapWheel | Off | Ignores key taps until turned off again or TapWheel restarts |

---

## Privacy

- No network connections: no account, no telemetry, no update checks.
- Files are handled on your PC by a bundled FFmpeg and background removal model, and stay in their folder.
- Errors are written to `%LOCALAPPDATA%\TapWheel\tapwheel.log` on your PC, so a problem can be found later. It never leaves your PC.
- The keyboard and mouse are watched only to spot a quick tap of Shift, Ctrl or Alt during a drag, or two quick taps on files selected in File Explorer.
- Nothing you type is recorded, and every key and click passes through untouched.

---

## Limitations

- Drags from apps running as administrator can't reach the wheel.
- Files Explorer shows inside a ZIP or on a phone need copying to a folder first.
- No documents, PDFs or Office files yet; they're planned for 0.3.
- MIDI files hold notes, not sound, so they aren't opened.
- AVIF and WebM are slow on large files.
- Tool windows need WebView2, which comes with Windows 10 and 11.

---

## Project Layout

```text
tapwheel/
├── crates/
│   ├── core/                 # Formats, tools, recipes, output names, the runner and every engine
│   ├── ui/                   # Draws the wheel, progress cards and icon
│   └── app/                  # The Windows app: keys, drags, wheel, cards, tray, tool windows
│       └── web/              # The tool windows and settings, shown in WebView2
├── engines/                  # Pinned FFmpeg, ONNX Runtime and model, and the script that fetches them
├── packaging/                # Portable zip and Store package builder, Store manifest and listing
├── site/                     # tapwheel.app, its sample files and its layout check
├── release-notes/            # One file per release
└── docs/PROJECT.md           # Project page
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

Licensed under the [GPL-3.0](LICENSE) license. Third-party parts are listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

---

## Links

- Website: https://tapwheel.app
- Microsoft Store: https://apps.microsoft.com/detail/9MW1GB4106VS
- Repository: https://github.com/muxover/tapwheel
- Issues: https://github.com/muxover/tapwheel/issues
- Changelog: [CHANGELOG.md](CHANGELOG.md)

---

<p align="center">Made with ❤️ by Jax (@muxover)</p>
