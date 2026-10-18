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
- 2026-10-18: everything TapWheel draws takes its look from Windows itself: the system accent shades, Segoe UI Variable on the Windows type ramp, 8 px corners on panels and 4 px on controls, Acrylic behind the wheel and Mica behind windows on Windows 11, Windows' motion curve and durations, and the two-ring focus outline. Windows 10 gets the same tokens, flat and square.
- 2026-10-18: wheel and ring labels follow the curve of their slice and never leave it. A test draws every wheel and fails if any letter falls outside its shape; a name that can't fit on two curved lines gets a shorter name.
- 2026-10-18: the website uses Windows blue as its accent, taken from the app, because its job is to show a Windows app.
- 2026-10-18: from the roadmap review, v0.1.0 adds Copy name and the other clipboard choices, Properties, Open with, Duplicate, Rename with a batch preview, File info, preflight checks, an Undo that checks the file first, and Retry and batch summaries on cards. Open containing folder is out (Explorer is already open where the drag started), file encryption is out until it has a security review (password ZIP and 7Z cover it), and there is no fourth key.
- 2026-10-18: the privacy page promises: the app makes no network connections; settings (keys, options, slice counts, recent Move to folders) stay in `%APPDATA%\TapWheel\settings.toml`; input is watched only for Shift, Ctrl, Alt, Tab and Esc and nothing typed is kept; the site sets no cookies and uses only Cloudflare Web Analytics. The zip version's tray menu has Remove from this PC, which deletes its right-click and start-up entries and its settings.
- 2026-10-18: the privacy page also promises: errors go to `%LOCALAPPDATA%\TapWheel\tapwheel.log` on the PC (error messages and file names, never typing, under 256 KB, removed by Remove from this PC, never sent anywhere).
- 2026-10-18: screenshots, screen recording and customizing the wheels are goals (asked for by Jax); they're planned for v0.2.0 next to Copy anything, which uses the same screen capture.
- 2026-10-18: the Contents window extracts what you tick instead of dragging single items out, which WebView2 can't hand to Explorer.
- 2026-10-18: the Store package can't write classic shell keys, so its right-click entry waits for the Explorer command handler in v0.2.0; the portable build adds a classic one.

## Status

v0.1.0 is being rebuilt from a converter into a full tool suite for seven kinds of files (below). The core engines pass about 200 jobs on real files in CI, the app compiles in CI, and the tool windows run on the website with sample files. Nothing of the new build has run on a Windows PC yet. Launch follows the phases in "Plan to launch".

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
| Add background | | Gradients, colours, any colour, your own picture, blur of the picture itself; padding, corners, shadow, ratio (Auto, 1:1, 16:9, 4:5, 9:16) |
| Remove background | Cut out, Touch up | Offline AI cut-out to transparent PNG; Touch up opens a window with Keep and Remove brushes that can follow edges, edge softness, and a fill behind |
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
| Snapshot | Middle, First, Every 10 s, Custom | Pick the exact frame |
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
- Tools: Extract (here, into a folder named after it, safe against path tricks), Contents (window with every file, size and date; tick files or folders to extract only those), Test (checks every file), Password (AES-256 ZIP or 7Z), Split (25 MB, 100 MB, 1 GB parts).
- Folders and mixed selections: pack into any of the formats above, or with a password.

### Actions (Alt wheel, any file or folder)

- Copy (to paste anywhere), Copy path (ring: full path, name, name without extension, extension, quoted path, file URI; several files give one per line), Rename (one file in place, several in a window with numbering, find and replace, case and date patterns, every new name previewed and conflicts shown before anything changes), Duplicate (a copy beside it with the next free name), File info (size, real type checked from the content, dates, picture size, streams and codecs, archive item count), Properties (the Windows properties window), Open with (the Windows app chooser), Move to (the four most recent folders on the ring, or choose one), Share (the Windows share window: Nearby Sharing, mail, apps), Float (a small shelf that holds files so you can drop them later), Checksum (SHA-256, MD5, SHA-1, or compare with a checksum on the clipboard), Terminal (folders and code only).

### Settings window (WebView2)

- Keys for each wheel, start with Windows, favourites first, pause, sounds.
- Defaults for every tool: quality per format, compress levels, video quality, audio bitrate, GIF size and speed, fade length, silence level, exact or instant trims, keep or remove photo details on convert, code shot style, background style, voice and speed.
- Sample files to practise on, the guide, privacy, source, and version.

### Quality bar for v0.1.0

- Every tool has an engine test with real files in CI on Windows, and every tool and editor is run by hand on a Windows 11 PC before launch.
- No tool blocks the wheel: work runs in the background and can be cancelled; a failed job leaves nothing behind.
- Before a job starts it checks free disk space, that the source can be read, and that its engine and model are present, and says what's wrong instead of starting. Leftover `.tw-part` files from a crash are cleared on start.
- Undo only removes a file TapWheel made and only if it hasn't changed since (size and modified time); otherwise the card says why.
- Cards show waiting, running, done, failed and cancelled; a failed job has Retry; a batch ends in one summary card with done, skipped and failed counts.
- Windows 10 and Windows 11 looks checked on both themes and high contrast; 100% to 200% scaling; several monitors.
- Website rebuilt with a live desktop where every wheel and editor works in the browser, a real clip per tool, and the full format list.

### Plan to launch

Each phase ends with CI green on main; nothing is tagged before G.

| Phase | Work | Done when |
| --- | --- | --- |
| A. Build | Last Windows lint errors; clean-code pass over the new code | The Windows app job passes in CI |
| B. Windows look and labels | The look and curved labels from Decisions | The label test passes for every wheel at 100%, 150% and 200%; screenshots on a Windows PC in both looks |
| C. Safety | Preflight, the checked Undo, crash leftovers, waiting and Retry, batch summaries | A test for each; Cancel and a forced failure leave no partial files |
| D. Alt actions | The new actions listed above | Tests in core; each tried by hand on a Windows PC |
| E. On a Windows PC | Every tool and window with real files; Windows 10 and 11; 100% to 200%; two monitors; on-screen keyboard and remote desktop; portable zip and Store package each on their own; network off | A checklist with a note or screenshot per line; every bug fixed with a test |
| F. Website | The redesign below, with real captures | The browser check passes; looked at on a phone |
| G. Release | README, CHANGELOG, release notes and these notes through the humanizer; notices checked; one final script signs, rewrites history to one commit per version, tags and releases | The release builds from a fresh clone; the Store package passes the App Certification Kit |

Progress: A is done. B, C and D are built and pass their tests in CI; the parts that need a Windows PC are checked in E.

Release blockers: a lost or replaced file, a claim the app can't back, a wheel that opens when it shouldn't or a stuck key, text outside its shape, a crash on a broken file, missing licence notices, a Store certification failure, or a wheel that waits on a job.

### Website for launch

Rebuilt after the app's look is settled. Windows blue accent, dark page. Sections spaced at about 64 px, words and a visual side by side so no screen is a heading alone. A playable hero: a large wheel on a strip of Windows desktop where visitors drag the sample file and tap Shift, Ctrl or Alt (key buttons on phones), with picture tools running in the browser. Order: hero, the three wheels, the tool windows to try, every kind of file, the safety promises, the Windows looks, coming next, questions, download. Short silent clips from a Windows PC per tool once the app runs. The tool list is generated from the app (`site/public/data/wheel.json`, checked in CI).

## v0.2.0: Copy anything, capture, your own wheels and screen readers

- Capture, from the same shortcut and the tray: a screenshot of an area, a window or the whole screen, opened straight in the Annotate window or copied; a screen recording of an area, a window or a screen to MP4 or GIF, with or without sound and the pointer, a countdown, and Stop from the tray or the shortcut again. Both save next to each other in a folder you choose, and every result gets a card with the same tools as any file.
- Your own wheels: hide, pin and reorder slices on each wheel; save a tool's settings as your own ring choice (for example "Compress to 2 MB"); pick which wheel each key opens; wheel size and the accent colour. Everything is in Settings, with a reset.
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

From the roadmap review, reordered so each version builds on the last. Plans, not promises.

| Version | Theme | Main work |
| --- | --- | --- |
| v0.2.0 | Copy anything, capture, your own wheels | Screenshots and screen recording; hide, pin and reorder slices; saved tool settings as ring choices |
| v0.5.0 | Automation | Multi-step recipes with preview, mixed-file batches that skip what doesn't fit, naming templates; recipes stay declarative, never shell commands |
| v0.6.0 | File intelligence | Compare text, pictures and binaries; duplicate finder with review and no automatic deletes; folder summaries; similar pictures later |
| v0.7.0 | Desktop layer | Command palette, quick preview, saved Float shelves, job history, per-monitor and per-device input settings |
| v0.8.0 | Sharing and extensions | Shareable recipe files inspected on import, community presets and themes; extensions only for trusted local tools, with permissions shown |
| v0.9.0 | Hardening | Compatibility matrix, fuzzing of parsers and archives, speed and memory on a low-end PC, upgrade and uninstall tests; fixes only |
| v1.0.0 | Stable | Stable recipe format, documented behaviour and limits, no known data-loss, security or accessibility bugs |

Never: cloud accounts, online conversion, an AI chat window, a plugin marketplace.

## Open questions

- Launch video: made later with Remotion and sound written in code, from real captures on a Windows 11 PC, when Jax says to make it.
