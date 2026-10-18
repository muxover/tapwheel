# TapWheel — maintainer notes

## What this is

TapWheel is a free, open-source Windows 10 and 11 app that converts, edits and acts on files without opening an app. You start dragging a file, tap Shift, Ctrl or Alt, and a wheel opens under the pointer. Dropping on a slice converts the file, runs a tool or opens a small editor, and the result lands next to the original. It's inspired by Tangerine for Mac and built natively for Windows, and it aims to be the most complete and polished tool of its kind on any platform.

## Stack and layout

- Rust workspace, GPL-3.0-or-later.
- `crates/core`: formats, tools, recipes, output naming, and the engines (FFmpeg runner, pictures, metadata, icons, text, code, archives, background removal). Plain Rust, tested on Linux and Windows.
- `crates/ui`: draws the wheel, cards and app icon with tiny-skia, so every surface renders to PNG in tests.
- `crates/app`: the Windows app on windows-sys: keyboard and mouse hooks, the layered wheel window as an OLE drop target, tray, cards, and the WebView2 windows for editors and settings.
- `crates/app/web`: the editor and settings pages (HTML, CSS, JS) shown in WebView2, also reused by the website's live demo.
- `engines/`: pinned FFmpeg and FFprobe build and the background-removal model (version, URL, SHA-256), fetched by `fetch.ps1`. Never committed.
- `packaging/`: portable zip and Store MSIX builder, Store manifest.
- `site/`: tapwheel.muxover.com.

## Decisions

- 2026-10-18: Rust and Win32 for everything that must be instant (hooks, wheel, cards). Editor and settings windows use WebView2, which ships with Windows 10 and 11, so editors get live previews and polished controls without bundling a browser.
- 2026-10-18: GPL-3.0-or-later with FFmpeg bundled, so everything works offline right after install. No tool may need an outside program or a network call in v0.1.0 or v0.2.0.
- 2026-10-18: the wheel opens on a tap (down and up within 400 ms, no other key between), never a hold. The drop target only ever answers COPY or NONE: Explorer reads MOVE as "delete the source".
- 2026-10-18: the hooks ignore only the input TapWheel sends itself, so the on-screen keyboard, voice control and automation tools work.
- 2026-10-18: originals are never changed. Outputs go next to the source as `name-suffix.ext`, written to `.tw-part` and renamed at the end; an existing file is never replaced.
- 2026-10-18: photo metadata is handled without re-encoding: removing location or all details keeps JPG, PNG and WebP pixels byte for byte, and conversions keep the date and camera unless the user turns that off.
- 2026-10-18: the UI follows the system: light or dark, accent colour, high contrast, Segoe UI Variable and rounded corners on Windows 11, Segoe UI and square corners on Windows 10. Icons come from Microsoft's Fluent UI System Icons (MIT), drawn by us so they match on both.
- 2026-10-18: Store identity `Jaxmuxover.TapWheel`, publisher `CN=50CBFD77-B27F-4D02-AB74-844DC5756318`, display name "Jax (muxover)".
- 2026-10-18: the Store package can't write classic shell keys, so its right-click entry waits for the Explorer command handler in v0.2.0; the portable build adds a classic one.

## Status

v0.1.0 is being rebuilt from a converter into a full tool suite for seven kinds of files (below). The first build was tested on a Windows 11 PC and the site is live at tapwheel.muxover.com; both get replaced before launch.

## How the wheel works (all versions)

- Three wheels on three keys, changeable in Settings: Shift converts, Ctrl opens tools, Alt opens actions. Tap another key on an open wheel to switch.
- Each slice has an icon and a label. The centre shows the file's name and type, and on hover what the slice will do and, where it can be known, the size of the result.
- Slices with choices grow an outer ring on hover (for example Compress: Balanced, Smaller, Best, Custom). Dropping on the slice uses the first choice; dropping on the ring uses that one. "Custom" opens the tool's window.
- More than eight slices: the last slice is More, and hovering it (or Tab, or the mouse wheel) turns to the next page.
- Tap the same key twice to repeat the last choice. Most-used slices move to the top.
- The wheel opens and closes with a quick scale and fade; on a drop the chosen slice pulses and the wheel folds away.
- Keyboard: Enter on a selected file in Explorer opens the wheel, arrows move, Enter applies, Esc cancels. Right-click entry "Open with TapWheel".
- Several files at once are handled in one go, with one card for the batch.

## Cards (all versions)

- Compact cards above the taskbar, on the monitor where you dropped, sized to their content (no empty button row).
- Progress with time left, Cancel, then Open, Show in folder and Undo when done; the size before and after for anything that shrinks.
- They slide in, stack without overlapping, and fade out after a few seconds unless the pointer is on them.

## v0.1.0: a master class in seven kinds of files

Every tool works offline with what ships inside TapWheel (FFmpeg, our own Rust engines, and Windows' built-in voices).

### Images

- Reads: JPG, PNG, WebP, AVIF, HEIC and HEIF, GIF (still and animated), TIFF, BMP, TGA, QOI, DDS, PSD (flattened), JPEG XL, JPEG 2000, OpenEXR, HDR, PCX, PPM family, and camera raw (DNG, CR2, CR3, NEF, ARW, ORF, RW2, RAF, PEF, SRW and more).
- Convert to: PNG, JPG, WebP, AVIF, GIF, TIFF, BMP, ICO (every Windows size), TGA, QOI. Animated GIF also to MP4, WebM and animated WebP.
- Tools:

| Tool | Ring choices | Window |
|------|--------------|--------|
| Crop | | Free or 1:1, 4:3, 3:2, 16:9, 9:16, 4:5; exact W×H; arrow-key nudging; full resolution kept |
| Add background | | Gradients, colours, blur of the picture itself; padding, corners, shadow, ratio (Auto, 1:1, 16:9, 4:5, 9:16) |
| Remove background | | Offline AI cut-out to transparent PNG; optional window to refine edges and pick a fill |
| Compress | Balanced, Smaller, Best, Custom | Quality, target size in KB, longest edge; result size shown live |
| Resize | 50%, 1920, 1280, 1080, Custom | Width, height, percent, keep proportions |
| Adjust | | Exposure, brightness, contrast, gamma, saturation, warmth, hue, sharpen; live preview; reset per slider |
| Annotate | | Arrow, box, highlight, pen, text, numbered steps, colours and sizes |
| Redact | | Blur, pixelate or black box, drawn over areas |
| Rotate | Right, Left, 180°, Mirror, Flip | |
| Metadata | Remove all, Remove location, View | Every field listed; remove one, location only, or all; without re-encoding |
| Make icon | | ICO with 16 to 256 |
| To SVG | | Trace into a vector |
| Combine (2 or more) | Grid, Side by side, Stacked, GIF, Custom | Layout, gap, background, order |

### Videos

- Reads: MP4, M4V, MOV, MKV, WebM, AVI, WMV, ASF, MPG, MPEG, TS, MTS, M2TS, 3GP, FLV, F4V, OGV, VOB, MXF, DV, RM, DivX.
- Convert to: MP4, MOV, MKV, WebM, GIF, MP3, AVI, WMV, M4A, WAV. Changing only the container (for example MKV to MP4 with H.264 and AAC) copies the streams in seconds with no quality loss.
- Tools:

| Tool | Ring choices | Window |
|------|--------------|--------|
| Trim | | Timeline with frames and sound wave; keep a part, cut a part out, or split at markers into separate files; exact or instant (keyframe) cuts |
| Compress | Balanced, Smaller, 25 MB, 10 MB, Custom | Quality, resolution, or an exact target size (two-pass) |
| Make GIF | | Part of the clip, frames per second, width; size shown |
| Resize | 720p, 1080p, 480p, 9:16 | |
| Crop | | Same crop window as images, on a frame of the video |
| Speed | 2×, 1.5×, 4×, 0.5× | Sound keeps its pitch |
| Snapshot | Middle, First, Every 10 s | Pick the exact frame |
| Mute, Get audio, Rotate, Reverse | Rotate: Right, Left, 180°, Mirror, Flip | |
| Join (2 or more) | | Order; different sizes fitted into the first one's frame |

### Audio

- Reads: MP3, M4A, M4B, AAC, WAV, FLAC, OGG, Opus, AIFF, WMA, ALAC, AMR, AC3, E-AC3, DTS, CAF, MKA, MP2, APE, WavPack, TTA, AU, W64, Speex.
- Convert to: MP3, M4A, WAV, FLAC, OGG, Opus, AIFF, WMA, iPhone ringtone (M4R).
- Tools:

| Tool | Ring choices | Window |
|------|--------------|--------|
| Trim | | Waveform with play, keep or cut parts, split into files |
| Fade | Both, In, Out | Length of each fade |
| Normalize | | Even loudness at -14 LUFS |
| Compress | 128k, 96k, 64k, 192k | |
| Volume | +6 dB, +3 dB, -3 dB, -6 dB | |
| Speed | 2×, 1.5×, 4×, 0.5× | Pitch kept |
| Metadata | Edit, Remove all | Title, artist, album, year, genre, track, comment, cover picture |
| Ringtone | | First 40 s with a fade, as M4R |
| Mono or stereo, Remove silence, Reverse | | |
| Join (2 or more) | | Order |

### Icons

- Reads: ICO, ICNS, CUR, SVG, SVGZ, and program files (EXE, DLL) for their icons.
- Convert to: PNG, ICO, ICNS, WebP, JPG. SVG renders crisp at any size.
- Tools: Windows icon (ICO with 16, 20, 24, 32, 40, 48, 64, 96, 128, 256), macOS icon (ICNS), PNG set (16 to 1024, or one size from the ring), Favicon pack (favicon.ico, 16, 32, 180 Apple touch, 192 and 512 Android, `site.webmanifest`, and the HTML to paste), Get icons (every size inside an ICO or ICNS, or the icon of a program, as PNGs), Minify SVG.

### Text

- Reads: TXT, Markdown, HTML, RTF, CSV, TSV, JSON, JSON Lines, YAML, TOML, XML, plist, INI, CFG, properties, .env, LOG, SRT, VTT, ASS, SSA, and other plain-text files.
- Convert to:
  - Data files (CSV, TSV, JSON, JSON Lines, YAML, TOML, XML) into each other, with nested data flattened for tables and rebuilt from them.
  - Config files (INI, properties, .env) to JSON, YAML, TOML.
  - Markdown to HTML (a styled page) and TXT; HTML to Markdown and TXT; RTF to TXT, Markdown, HTML.
  - Subtitles: SRT, VTT, ASS, SSA to SRT, VTT, TXT.
- Tools: Read aloud (MP3 with any Windows voice and speed), Format and Minify (JSON, XML, YAML, TOML, HTML, CSV), Fix encoding to UTF-8 (detects the original), Clean up (trailing spaces, blank runs, tabs), Sort lines (A to Z, Z to A, numbers, length), Remove duplicates, Change case (Title, UPPER, lower, Sentence), Line endings (Windows, Unix), Split (1,000, 100 or 10,000 lines), Merge (2 or more), Count (lines, words, characters, reading time, copied on click).

### Code

- Reads: Rust, Python, JavaScript, TypeScript, JSX, TSX, Go, C, C++, C#, Java, Kotlin, Swift, Objective-C, Ruby, PHP, Perl, Lua, R, Julia, Dart, Zig, Elixir, Erlang, Haskell, Clojure, OCaml, Shell, PowerShell, Batch, SQL, CSS, SCSS, Less, Vue, Svelte, Astro, GraphQL, Protobuf, Dockerfile, Makefile, CMake, Terraform, Nix, shaders, assembly, Verilog, Solidity, GDScript, AutoHotkey, .reg.
- Convert to: PNG (a code shot with the default style), HTML (highlighted page), Markdown (fenced block), TXT. JSON also to YAML, TOML, XML, CSV.
- Tools: Code shot (window with themes, backgrounds, padding, line numbers, window frame, title, font size), Copy as Markdown, Format and Minify (JSON, CSS, SCSS, Less, HTML, XML, SQL, JavaScript; Vue, Svelte and Astro format), Indent (tabs, 2 or 4 spaces), Line endings, Clean up, Fix encoding, Merge.

### Archives

- Reads: ZIP, 7Z, TAR, TAR.GZ, TGZ, TAR.BZ2, TAR.XZ, TAR.ZST, GZ, BZ2, XZ, ZST, and ZIP-based files (JAR, APK, CBZ, NUPKG, VSIX, WHL).
- Convert to: ZIP, 7Z, TAR.GZ, TAR.XZ, TAR, TAR.BZ2, TAR.ZST.
- Tools: Extract (here, into a folder named after it, safe against path tricks), Contents (window with every file, size and date; extract single items by dragging them out), Test (checks every file), Password (AES-256 ZIP or 7Z), Split (25 MB, 100 MB, 1 GB parts).
- Folders and mixed selections: pack into any of the formats above, or with a password.

### Actions (Alt wheel, any file or folder)

- Copy (to paste anywhere), Copy path, Move to (the four most recent folders on the ring, or choose one), Share (the Windows share window: Nearby Sharing, mail, apps), Float (a small shelf that holds files so you can drop them later), Checksum (SHA-256, MD5, SHA-1, or compare with a checksum on the clipboard), Terminal (folders and code only).

### Settings window (WebView2)

- Keys for each wheel, start with Windows, favourites first, pause, sounds.
- Defaults for every tool: quality per format, compress levels, video quality, audio bitrate, GIF size and speed, fade length, silence level, exact or instant trims, keep or remove photo details on convert, code shot style, background style, voice and speed.
- Sample files to practise on, the guide, privacy, source, and version.

### Quality bar for v0.1.0

- Every tool has an engine test with real files in CI on Windows, and every tool and editor is run by hand on a Windows 11 PC before launch.
- No tool blocks the wheel: work runs in the background and can be cancelled; a failed job leaves nothing behind.
- Windows 10 and Windows 11 looks checked on both themes and high contrast; 100% to 200% scaling; several monitors.
- Website rebuilt with a live desktop where every wheel and editor works in the browser, a real clip per tool, and the full format list.

## v0.2.0: Copy anything and screen readers

- Copy anything: a shortcut (and a tray item) freezes the screen; drag over any area (a paused video, a picture, a PDF page, an app that won't let you select) and its text is copied with the offline Windows OCR, in the languages Windows has installed. Tables come out as tab-separated rows.
- QR and barcodes: the same capture reads QR codes and common barcodes; links open or copy, Wi-Fi codes show the network and password, text is copied.
- From the wheel: images get Copy text and Read QR; videos get Copy text from the current frame chosen in a frame picker.
- Make QR: any text or link file, or the clipboard, into a QR PNG or SVG.
- Screen readers: UI Automation for the wheel, cards, editors and settings, with announcements for wheel changes, hover and results; full keyboard use everywhere.
- Store right-click: an Explorer command handler, which also puts TapWheel in the top-level Windows 11 menu, for both the Store and portable builds.

## v0.3.0: documents

- PDF: merge, split, reorder, rotate and delete pages (thumbnails window), compress, to images (PNG, JPG at 150 or 300 DPI), images to PDF, extract text, extract images, add page numbers and watermarks, password protect and remove a known password, fill and flatten forms, sign with a drawn or typed signature. Engine: a bundled PDFium.
- Microsoft Office, every version: DOC, DOCX, DOCM, DOT, DOTX, RTF, XLS, XLSX, XLSM, XLSB, CSV, PPT, PPTX, PPSX, POT, POTX, and OpenDocument (ODT, ODS, ODP). Convert to PDF, to each other's newer or older format, DOCX to Markdown, HTML and TXT, XLSX to CSV and JSON per sheet, PPTX to images per slide. Tools: compress embedded pictures, remove personal details, extract images, count words.
- Google Docs, Sheets and Slides: their downloaded formats (DOCX, XLSX, PPTX, ODT, PDF) get everything above; `.gdoc`, `.gsheet` and `.gslides` shortcuts open the file in the browser, since the document itself lives online.
- EPUB: to PDF and TXT, cover extraction, metadata.
- Engine choice, checked on first use: Microsoft Office when installed (exact layouts), otherwise an optional one-click LibreOffice download. That is TapWheel's first network call, so the privacy page and the settings explain it and it never happens without a click.

## v0.4.0: local AI

- Every AI feature first checks the PC (memory, graphics card, CPU features, free disk) and only offers what will run smoothly; models download on demand with their size shown, and run fully offline after that.
- Transcribe: audio and video to text, SRT and VTT subtitles, with Whisper (whisper.cpp) in the size the PC can handle; burn the subtitles into a video.
- Text tools with a very small local language model (llama.cpp): rewrite, fix grammar, change tone (formal, friendly, short), summarize, translate, and click to reply to an email (`.eml`, `.msg`, or text on the clipboard), which drafts a reply to edit and copy.
- Picture upscaling (2× and 4×) where the graphics card can do it quickly.

## Later

Not planned yet.

## Open questions

- Launch video: made later with Remotion and sound written in code, from real captures on a Windows 11 PC, when Jax says to make it.
