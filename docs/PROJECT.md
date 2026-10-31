# TapWheel project

TapWheel is a free, open-source Windows app that converts, edits and acts on files without opening an app. Start dragging a file, tap Shift, Ctrl or Alt, and a wheel opens under the pointer. Drop the file on a slice and it's converted, edited in a small window, or acted on, and the result lands next to the original or in the folder you choose. Everything runs on your PC. It's inspired by Tangerine for Mac and built natively for Windows, and it stays free.

This page is for anyone who wants to understand how TapWheel works, what it does today, where it's going, or how to help. To use it, see the [README](../README.md) or [tapwheel.app](https://tapwheel.app).

## What TapWheel is for

The fastest way to do one thing to a file on Windows: drag, tap, drop, done, offline. TapWheel lives in the moment between picking a file up and putting it down. It isn't a file manager, a screenshot tool, a launcher or an AI assistant.

Every feature has to pass five rules, so the wheel stays quick to read:

1. It starts from a file you're dragging or have selected.
2. It ends as a file next to the original or in the folder you choose, or on the clipboard.
3. It takes one drop or one small window.
4. It works offline. The only exception is a one-click download from an official source.
5. It fits on the wheel: at most eight slices on a kind of file's first page. A new tool goes on a ring, under More, or replaces a weaker one.

TapWheel also has to stay smooth on the PC it runs on, low-end laptops included, so every heavy tool has to know what it costs.

TapWheel will never have cloud accounts, online conversion, an AI chat window or a plugin marketplace.

## How it works

### The wheel

- Three wheels on three keys, which you can change in Settings: Shift converts, Ctrl opens tools, Alt opens actions. Tapping another key on an open wheel switches to that wheel.
- The wheel opens on a quick tap (down and up within 400 ms, no other key in between), never on a hold, while files are being dragged.
- Without a drag, two quick taps of a wheel's key open that wheel beside the files selected in File Explorer or on the desktop. Settings can change this to Ctrl+Shift+Space or turn it off. It only works while the file list has the keyboard, so typing in a rename box, the address bar or the search box is never caught.
- Each slice has an icon and a label that follows the slice's curve. The centre shows the file's name and type, and on hover what the slice will do and, where it can be known, the size of the result.
- Slices with choices grow an outer ring on hover, for example Compress: Balanced, Smaller, Best, Custom. Dropping on the slice uses the first choice, dropping on the ring uses that one, and Custom opens the tool's window.
- With more than eight slices the last one becomes More. Hover it, press Tab or roll the mouse wheel to turn the page.
- Tap the same key twice to repeat your last choice. The slices you use most move to the top.
- The mark in the tray and on TapWheel's windows takes the accent colour (Windows' or the one picked in Settings) and the taskbar's light or dark, and changes with them. Start, the taskbar and the Store, which can't take an accent, get the brand blue on a dark or light disc to match the taskbar.
- The wheel fits whichever screen the pointer is on: it takes that monitor's scaling, shrinks to fit a small work area (1366×768 at 125%, for one) and never crosses onto a neighbouring monitor.
- Results go next to the original by default. Settings can put them in a subfolder beside it or in one folder instead; a duplicate always stays next to its original.
- A batch rename has Undo on its card, which puts every old name back or, if a file was renamed again since, none.
- Each wheel can be arranged for each kind of file in Settings: pin slices to the front in your order, hide the ones you never use, and keep a tool's settings as a choice on its ring (up to three per slice, before Custom). The wheel's size and colour can be changed too; its colour follows the Windows accent until you pick one.
- Several files are handled in one go, with one card for the batch. Folders and mixed selections get the Actions wheel.
- TapWheel is in Explorer's right-click menu; on Windows 11 the Store version's entry is at the top of the menu. Choosing it opens the Convert wheel on the selection, ready for the keyboard.

### Fits your PC

When it starts, TapWheel reads the PC once: processor cores, memory, the graphics card, whether the drive for work in progress is an SSD, battery and Battery saver, and the screen's refresh rate. It runs one batch at a time on PCs with 8 GB of memory or less, two cores or a hard drive, two on mid-range PCs and three on roomy ones. Video work and background removal wait while Battery saver is on and say so on their card. Without a graphics driver, or with Windows animations off, the wheel opens and closes without animation. Settings, General shows what it read.

At start TapWheel also tries a one-second test encode with the graphics card's own H.264 encoder (NVIDIA NVENC, Intel Quick Sync or AMD AMF). When one works, Compress on Balanced uses it and the card names it: much faster, with files a little bigger. Smaller and size targets stay on the processor, which packs more into each megabyte, and a card that turns a job down hands it to the processor. Running cards show the time left once enough has run to tell, and say plainly when a job will be slow on this PC.

### Screen readers and the keyboard

The wheel and the cards are drawn by TapWheel, so they speak to UI Automation themselves: Narrator, NVDA and JAWS hear the wheel open with its file and choices, each slice or ring choice as the pointer or keyboard reaches it, page turns, and every finished or failed job. On a wheel opened without a drag, Left and Right move between slices, Down opens a slice's ring choices and moves along them, Up comes back, Enter picks, number keys pick directly and Esc closes. The newest job's buttons (Open, Show in folder, Undo, Retry) are also in the tray menu, which Win+B reaches. The tool windows and Settings are web pages built for keyboards and screen readers: one tab stop per choice group with the arrow keys inside it, headings in order, and every picture named.

### Cards

- Compact cards above the taskbar, on the monitor where you dropped the file.
- They show progress with the time left and Cancel, then Open, Show in folder and Undo when done, with the size before and after for anything that shrinks.
- They slide in, stack without overlapping, and fade out after a few seconds unless the pointer is on them. A batch ends in one card with done, skipped and failed counts, and a failed job has Retry.

### Your files stay safe

- Originals are never changed. Results go next to the source as `name-suffix.ext`, are written to a `.tw-part` file first and renamed at the end, and never replace an existing file.
- The drop only ever copies. Explorer reads a "move" answer as "delete the source", so TapWheel never gives one.
- Before a job starts it checks free space, free memory, that the file can be read and that its engine is present. If something's wrong, it says what instead of starting.
- Undo removes only a file TapWheel made, and only if you haven't changed it since. Cancel keeps nothing.
- Removing a photo's location or details doesn't re-encode it: JPG, PNG and WebP pixels stay byte for byte the same.
- After 30 idle seconds TapWheel gives memory back to Windows.

### It looks like Windows

Everything TapWheel draws takes its look from Windows: your accent colour, light or dark mode, high contrast, Segoe UI Variable on the Windows type ramp, rounded corners, Acrylic behind the wheel and Mica behind windows where Windows has them, and Windows' own motion. Icons come from Microsoft's Fluent UI System Icons.

## What it does

Every tool works offline with what ships inside TapWheel: FFmpeg, TapWheel's own Rust engines, a background-removal model, and the voices built into Windows.

### Pictures

- Reads JPG, PNG, WebP, AVIF, HEIC and HEIF, GIF (still and animated), TIFF, BMP, TGA, QOI, DDS, PSD (flattened), JPEG XL, JPEG 2000, OpenEXR, HDR, PCX, the PPM family, and camera raw (DNG, CR2, CR3, NEF, ARW, ORF, RW2, RAF, PEF, SRW and more).
- Converts to PNG, JPG, WebP, AVIF, GIF, TIFF, BMP, ICO, TGA and QOI. Animated GIFs also become MP4, WebM or animated WebP.

| Tool | Ring choices | Window |
| --- | --- | --- |
| Crop | | Free or a fixed ratio, exact width and height, arrow-key nudging |
| Add background | | Gradients, any colour, your own picture or a blur of the photo; padding, corners, shadow, ratio |
| Remove background | Cut out, Touch up | Offline AI cut-out to a transparent PNG; Touch up has Keep and Remove brushes that can follow edges |
| Compress | Balanced, Smaller, Best, Custom | Quality, a target size in KB, longest edge, with the result size shown live |
| Resize | 50%, 1920, 1280, 1080, Custom | Width, height, percent, keep proportions |
| Adjust | | Exposure, brightness, contrast, gamma, saturation, warmth, hue, sharpen, with a live preview |
| Annotate | | Arrow, box, highlight, pen, text, numbered steps |
| Redact | | Blur, pixelate or black box over any area |
| Rotate | Right, Left, 180°, Mirror, Flip | |
| Metadata | Remove all, Remove location, View | Every field listed, removed one by one or all at once |
| Make icon | | ICO from 16 to 256 px |
| To SVG | | Traces the picture into a vector |
| Combine | Grid, Side by side, Stacked, GIF, Custom | Layout, gap, background, order |

### Videos

- Reads MP4, M4V, MOV, MKV, WebM, AVI, WMV, ASF, MPG, MPEG, TS, MTS, M2TS, 3GP, FLV, F4V, OGV, VOB, MXF, DV, RM and DivX.
- Converts to MP4, MOV, MKV, WebM, GIF, MP3, AVI, WMV, M4A and WAV. When only the container changes, the streams are copied in seconds with no quality loss. On Windows on Arm, WebM is made with AV1 instead of VP9, and AVIF pictures with the rav1e encoder.

| Tool | Ring choices | Window |
| --- | --- | --- |
| Trim | | Timeline with frames and the sound wave; keep a part, cut a part out, or split at markers; exact or instant cuts |
| Compress | Balanced, Smaller, 25 MB, 10 MB, Custom | Quality, resolution, or an exact target size |
| Make GIF | | Part of the clip, frames per second, width, with the size shown |
| Resize | 720p, 1080p, 480p, 9:16 | |
| Crop | | The crop window, on a frame of the video |
| Speed | 2×, 1.5×, 4×, 0.5× | Sound keeps its pitch |
| Snapshot | Middle, First, Every 10 s, Custom | Pick the exact frame |
| Mute, Get audio, Rotate, Reverse | Rotate: Right, Left, 180°, Mirror, Flip | |
| Join | | Order; different sizes fitted into the first one's frame |

### Sound

- Reads MP3, M4A, M4B, AAC, WAV, FLAC, OGG, Opus, AIFF, WMA, ALAC, AMR, AC3, E-AC3, DTS, CAF, MKA, MP2, APE, WavPack, TTA, AU, W64 and Speex.
- Converts to MP3, M4A, WAV, FLAC, OGG, Opus, AIFF, WMA and iPhone ringtones (M4R).

| Tool | Ring choices | Window |
| --- | --- | --- |
| Trim | | Waveform with play; keep or cut parts, or split into files |
| Fade | Both, In, Out | Length of each fade |
| Normalise | | Even loudness at -14 LUFS |
| Compress | 128k, 96k, 64k, 192k | |
| Volume | +6 dB, +3 dB, -3 dB, -6 dB | |
| Speed | 2×, 1.5×, 4×, 0.5× | Pitch kept |
| Metadata | Edit, Remove all | Title, artist, album, year, genre, track, comment, cover |
| Ringtone | | The first 40 s with a fade, as M4R |
| Mono or stereo, Remove silence, Reverse | | |
| Join | | Order |

### Icons

- Reads ICO, ICNS, CUR, SVG and SVGZ, and the icons inside programs (EXE, DLL).
- Converts to PNG, ICO, ICNS, WebP and JPG. SVG renders sharp at any size.
- Tools: Windows icon (ICO from 16 to 256 px), macOS icon (ICNS), PNG set (16 to 1024 px), favicon pack (with `site.webmanifest` and the HTML to paste), Get icons (every size inside an icon or a program, as PNGs), and Minify SVG.

### Text

- Reads TXT, Markdown, HTML, RTF, CSV, TSV, JSON, JSON Lines, YAML, TOML, XML, plist, INI, CFG, properties, .env, LOG, SRT, VTT, ASS, SSA and other plain-text files.
- Converts data files (CSV, TSV, JSON, JSON Lines, YAML, TOML, XML) into each other, config files (INI, properties, .env) to JSON, YAML and TOML, Markdown to HTML and TXT, HTML to Markdown and TXT, RTF to TXT, Markdown and HTML, and subtitles between SRT, VTT, ASS and SSA.
- Tools: Read aloud (an MP3 in any Windows voice), Format and Minify (JSON, XML, YAML, TOML, HTML), Fix encoding to UTF-8, Clean up, Sort lines, Remove duplicates, Change case, Line endings, Split, Merge and Count.

### Code

- Reads more than 50 languages and formats, including Rust, Python, JavaScript, TypeScript, Go, C, C++, C#, Java, Kotlin, Swift, PHP, Ruby, Shell, PowerShell, SQL, CSS, HTML, Vue, Svelte, Dockerfile and Terraform.
- Converts to a PNG code shot, a highlighted HTML page, a fenced Markdown block or TXT. JSON also converts to YAML, TOML, XML and CSV.
- Tools: Code shot (themes, backgrounds, padding, line numbers, window frame, title, font size), Copy as Markdown, Format and Minify, Indent, Line endings, Clean up, Fix encoding and Merge.

### Archives

- Reads ZIP, 7Z, TAR, TAR.GZ, TGZ, TAR.BZ2, TAR.XZ, TAR.ZST, GZ, BZ2, XZ, ZST, and ZIP-based files such as JAR, APK, CBZ, NUPKG, VSIX and WHL.
- Converts to ZIP, 7Z, TAR.GZ, TAR.XZ, TAR, TAR.BZ2 and TAR.ZST.
- Tools: Extract (safe against path tricks), Contents (tick only the files you want), Test, Password (AES-256 ZIP or 7Z) and Split into parts. Folders and mixed selections can be packed into any of these formats.

### Actions, for any file or folder

Copy, Copy path (full path, name, name without extension, extension, quoted path or file URI), Rename (one file, or many with numbering, find and replace, and every new name previewed before anything changes), Duplicate, File info (with the real type checked from the content), Properties, Open with, Move to, Share, Float (a small shelf that holds files to drop later) and Checksum.

### Settings

Keys for each wheel, how the wheel opens without a drag, how each wheel is arranged with its size and colour, your ring choices, start with Windows, favourites first, pause, and defaults for every tool. Settings also has sample files to practise on.

## Privacy

The app makes no network connections: no account, no telemetry, no crash reports, no update checks. Files never leave your PC. TapWheel watches the keyboard only to notice Shift, Ctrl, Alt, Tab and Esc during a drag, and double taps of Shift, Ctrl or Alt on files selected in File Explorer. It keeps no record of what you type and passes every other key on unchanged. On a double tap it asks File Explorer which files are selected, and on a double tap or the right-click entry where the selected one is on screen; it asks only at that moment and keeps nothing. Errors go to a log on your PC that is never sent anywhere. Report a problem in Settings opens a GitHub issue in your browser, filled in with the version, Windows, the screens and the end of the log with your user name taken out; nothing leaves the PC unless you post it. The full policy is at [tapwheel.app/privacy](https://tapwheel.app/privacy).

## How it's built

- A Rust workspace, licensed GPL-3.0-or-later, built natively for x64 and for Windows on Arm, each with the engines built for it.
- `crates/core`: formats, tools, output naming and the engines (the FFmpeg runner, pictures, metadata, icons, text, code, archives, background removal). Plain Rust, tested on Linux and Windows.
- `crates/ui`: draws the wheel, cards and app icon with tiny-skia, so every surface renders to PNG in tests.
- `crates/app`: the Windows app: keyboard and mouse hooks on their own thread, the wheel as a layered drop-target window, the tray, cards, and WebView2 windows for the editors and settings.
- `crates/app/web`: the editor and settings pages. The website uses its wheel demo, and `site/public/try/` runs the editors with sample files to make the screens the website shows.
- `crates/shellext`: a small DLL that puts TapWheel in Explorer's right-click menu for the Store version, since a packaged app can't write the classic menu keys. It hands the selected files to the running app.
- `engines/`: the pinned FFmpeg build and background-removal model (version, URL and SHA-256), fetched by `fetch.ps1` and never committed.
- `packaging/`: builds the portable zip and the Microsoft Store package.
- `site/`: the website, [tapwheel.app](https://tapwheel.app).

The wheel, cards and hooks are native Win32, because they have to be instant. The tool windows use WebView2, which ships with Windows, so they get live previews without bundling a browser. FFmpeg and the model ship inside the app, so everything works right after install with no network.

Every tool has a test with real files in CI, on Linux and Windows. Each fix in a patch release comes with a test that fails without it.

## Releases

| Version | Date | What it brought |
| --- | --- | --- |
| [v0.1.0](https://github.com/muxover/tapwheel/releases/tag/v0.1.0) | 2026-10-18 | The first release: three wheels and seven kinds of files |
| [v0.1.1](https://github.com/muxover/tapwheel/releases/tag/v0.1.1) | 2026-10-23 | Bug fixes: steadier on big files, no crashes on broken inputs, right output everywhere, memory given back when idle |
| [v0.2.0](https://github.com/muxover/tapwheel/releases/tag/v0.2.0) | RELEASE-DATE | Reach and control: the wheel without dragging, your own wheels, graphics card video, screen readers and a native Arm build |

The full list is in the [CHANGELOG](../CHANGELOG.md). Patch releases (v0.x.1 and up) only fix bugs; new features come in the next minor version.

## Roadmap

Each version has one theme and builds on the last.

### v0.3.0: languages

Every string comes from translation files. Arabic comes first, right to left throughout, then the languages people ask for. Anyone can add a language with one file and a pull request.

### v0.4.0: documents

PDF becomes the eighth kind of file: merge, split, reorder, rotate and delete pages, compress, convert to and from pictures, extract text and pictures, page numbers, watermarks, passwords, flatten forms and sign. Office, OpenDocument and EPUB files convert to PDF, Word files to Markdown or TXT, and spreadsheets to CSV. Office files use Microsoft Office when it's installed, or LibreOffice downloaded from its official site with one click.

### v0.5.0: smart and offline

Transcribe audio and video to subtitles and burn them in, upscale pictures, copy text out of a picture with Windows' own OCR, and read or make QR codes. Each feature checks the PC first, and models download on demand with their size shown, then run fully offline.

### v0.6.0: batches and recipes

Multi-step recipes with a preview, saved and shared as files, one drop on a mix of different kinds of files, naming templates, and comparing two files.

### v0.7.0 and v1.0.0

v0.7.0 is for hardening only: compatibility, fuzzing, speed on a low-end PC, and upgrade and uninstall tests. v1.0.0 is the stable release, with a stable recipe format, documented limits and no known data-loss, security or accessibility bugs.

## Contributing

Bug reports, ideas and pull requests are welcome. Start with [CONTRIBUTING.md](../CONTRIBUTING.md), and report security issues as [SECURITY.md](../SECURITY.md) describes. A bug report helps most with the TapWheel version, whether it's the Store or the zip version, the kind of file, and what you expected to happen.
