# Microsoft Store listing

What goes into Partner Center for TapWheel (Store ID 9MW1GB4106VS). Paste each part into the field it's named after. Every claim here is something the app does in v0.2.0.

## Product name

TapWheel

## Short description

The file wheel for Windows. Drag a file, tap Shift, Ctrl or Alt, and drop it on the wheel to convert, crop, trim, compress or remove a background. Free and offline.

## Description

TapWheel puts file tools on a wheel that opens while you drag. Start dragging a file in File Explorer or on the desktop, tap a key, and drop it on what you want. The new file lands next to the old one, or in a folder you choose. No drag needed either: select files and tap the key twice, or right-click them and choose TapWheel.

Shift opens Convert: pick a format for pictures, video, audio, icons, text, code or archives. Ctrl opens Tools: crop, trim, compress, resize, remove the background, make GIFs, rotate, add a backdrop and more, with only the tools that fit the file you're holding. Alt opens Actions: copy, rename, move, duplicate, share, file info, Properties and Open with.

Tools that need setting up open their own window with a live preview, like Crop, Trim, Annotate, Redact, Add background and Touch up for cut-outs. Rest on a slice for quick choices on an outer ring, like Balanced or 25 MB, and keep your own settings as a choice there too. Pin, hide and reorder slices, and pick the wheel's size and colour.

TapWheel fits the PC it runs on. Compress on Balanced uses the graphics card's video encoder when there is one, long jobs show the time left, and heavy work waits while Battery saver is on. It runs natively on Arm PCs as well as Intel and AMD, and screen readers hear the wheel and every finished job.

Everything runs on your PC. TapWheel makes no network connections: no account, no uploads, no ads and no telemetry. Background removal uses an AI model that ships inside the app.

TapWheel is free and open source under the GPL-3.0 license.

## What's new in this version

Open the wheel without dragging: select files and tap a wheel's key twice, or use TapWheel in the right-click menu. Arrange your own wheels, keep a tool's settings as a ring choice, and choose where results go. Faster video compression on the graphics card, time left on long jobs, Undo for batch rename, screen reader support, a native Arm version and Report a problem.

## Product features

Up to 20, one per line.

- Drag a file, tap Shift, Ctrl or Alt, drop it on the wheel
- Or select files and tap the key twice, no drag needed
- TapWheel in File Explorer's right-click menu
- Converts pictures, video, audio, icons, text, code and archives
- Opens 48 picture formats, including HEIC, JPEG XL, PSD and camera RAW
- Crop, resize, compress, rotate, adjust and annotate pictures
- Removes backgrounds on your PC, with brushes to touch up the cut-out
- Trims video and sound on a timeline, with several cuts at once
- Compresses video to a quality or a size, Balanced on the graphics card
- Makes GIFs, saves frames and pulls the sound out of a video
- Makes Windows icons, macOS icons, favicon packs and PNG sets
- Formats JSON, XML, YAML, CSS and HTML, and makes code shots
- Extracts, tests, splits and password-protects ZIP and 7Z
- Renames many files at once, with a preview and Undo
- Your own wheels: pin, hide and reorder slices, pick size and colour
- Results next to the original, in a subfolder or in one folder
- Result cards with time left, Undo, Retry and Cancel
- Works with screen readers and the keyboard
- Follows light and dark mode, your accent colour and high contrast
- Works offline: no account, no uploads, no telemetry, free and open source

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

- Privacy policy URL: https://tapwheel.app/privacy
- Website: https://tapwheel.app/
- Support contact: https://tapwheel.app/support

## Category

Productivity, subcategory none. Also fits Utilities and tools.

## Notes for certification

TapWheel is a desktop app (full trust). It watches the keyboard and mouse with low-level hooks for one reason: to notice a quick tap of Shift, Ctrl or Alt, plus Tab and Esc, while a file is being dragged, and two quick taps of one of those keys on files selected in File Explorer. It records nothing and sends nothing. To try it: run TapWheel, start dragging any picture in File Explorer, and tap Shift while still holding the mouse button. Or select the picture and tap Shift twice.

The package adds one File Explorer context menu entry, TapWheel, through a packaged IExplorerCommand handler (a COM surrogate DLL in the package). Choosing it opens the Convert wheel on the selected files.

It starts with Windows by default (the user can turn this off in its window), so the wheel is ready when a drag begins.
