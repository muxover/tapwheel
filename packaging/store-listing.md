# Microsoft Store listing

What goes into Partner Center for TapWheel (Store ID 9MW1GB4106VS). Paste each part into the field it's named after. Every claim here is something the app does in v0.1.0.

## Product name

TapWheel

## Short description

Drag a file, tap Shift, Ctrl or Alt, and drop it on a wheel to convert, crop, trim, compress or remove a background. Free and offline.

## Description

TapWheel puts file tools on a wheel that opens while you drag. Start dragging a file in File Explorer or on the desktop, tap a key, and drop it on what you want. The new file lands next to the old one.

Shift opens Convert: pick a format for pictures, video, sound, icons, text, code or archives. Ctrl opens Tools: crop, trim, compress, resize, remove the background, make GIFs, rotate, add a backdrop and more, with only the tools that fit the file you're holding. Alt opens Actions: copy, rename, move, duplicate, share, file info, Properties and Open with.

Tools that need setting up open their own window with a live preview, like Crop, Trim, Annotate, Redact, Add background and Touch up for cut-outs. Rest on a slice for quick choices on an outer ring, like Balanced or 25 MB.

Everything runs on your PC. TapWheel makes no network connections: no account, no uploads, no ads and no telemetry. Background removal uses an AI model that ships inside the app.

TapWheel is free and open source under the GPL-3.0 license.

## What's new in this version

The first release.

## Product features

Up to 20, one per line.

- Drag a file, tap Shift, Ctrl or Alt, drop it on the wheel
- Converts pictures, video, audio, icons, text, code and archives
- Opens 48 picture formats, including HEIC, JPEG XL, PSD and camera RAW
- Crop, resize, compress, rotate, adjust and annotate pictures
- Removes backgrounds on your PC, with brushes to touch up the cut-out
- Trims video and sound on a timeline, with several cuts at once
- Compresses video to a quality or a size like 25 MB
- Makes GIFs, saves frames and pulls the sound out of a video
- Windows icons, macOS icons, favicon packs and PNG sets
- Formats and minifies JSON, XML, YAML, CSS, HTML and more
- Code shots: pictures of your code with themes and backdrops
- Extracts, tests, splits and password-protects ZIP and 7Z
- Rename many files at once with every new name shown first
- Shows a file's real type, even when its name says otherwise
- Result cards with Undo, Retry and Cancel
- Never replaces an existing file
- Follows light and dark mode, your accent colour and high contrast
- Works offline: no account, no uploads, no telemetry
- Free and open source

## Keywords

Up to 7. Another app's name (like Tangerine) isn't allowed as a keyword.

- file converter
- image converter
- video converter
- remove background
- compress video
- drag and drop
- file tools

## Screenshots

Upload `packaging/store/tapwheel-1.png` to `tapwheel-7.png` in this order (1920 × 1080 PNG). Each is a headline beside the real app: the wheels are drawn by TapWheel's own code from its real tool lists (`cargo run -p tapwheel-ui --example store_wheels`), and the windows are the app's own pages with the site's sample files. Nothing in them is mocked up or generated.

1. Drag a file. Tap a key. (the Tools wheel, Compress's ring open)
2. Convert to any format (the Convert wheel on a HEIC photo)
3. Remove backgrounds on your PC (Touch up)
4. Trim video and sound (Trim)
5. Tools with a live preview (Crop)
6. Actions for any file (the Actions wheel, Copy path's ring open)
7. Code shots in one drop (Code shot)

Redo them when a wheel or window changes, so the listing never shows something the app doesn't.

## Store logos and listing art

The package already carries its tiles and icons, drawn from the app's own mark. A 2160 × 1080 hero image is optional and can wait.

## Support and privacy

- Privacy policy URL: https://tapwheel.muxover.com/privacy
- Website: https://tapwheel.muxover.com/
- Support contact: https://tapwheel.muxover.com/support

## Category

Productivity, subcategory none. Also fits Utilities and tools.

## Notes for certification

TapWheel is a desktop app (full trust). It watches the keyboard and mouse with low-level hooks for one reason: to notice a quick tap of Shift, Ctrl or Alt, plus Tab and Esc, while a file is being dragged. It records nothing, sends nothing, and passes every key and click on unchanged. To try it: run TapWheel, start dragging any picture in File Explorer, and tap Shift while still holding the mouse button.

It starts with Windows by default (the user can turn this off in its window), so the wheel is ready when a drag begins.
