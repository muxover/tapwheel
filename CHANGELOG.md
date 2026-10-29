# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Select files in File Explorer or on the desktop and tap a wheel's key twice to open that wheel without dragging. Settings can switch it to Ctrl+Shift+Space or turn it off.
- The Store version is in Explorer's right-click menu too, at the top of the menu on Windows 11.
- Wheels page in Settings: arrange each wheel for each kind of file by pinning, hiding and moving slices, and pick the wheel's size and colour.
- TapWheel reads the PC when it starts and fits its work to it: how many batches run at once, video and background removal waiting while Battery saver is on, and a lighter wheel animation without a graphics driver. Settings, General shows what it read.
- Compress on Balanced uses the graphics card's video encoder when this PC has one, named on the card. Settings can turn it off.
- Cards show the time left on long jobs, and say when a job will be slow on this PC.
- Screen readers hear the wheel, its slices and ring choices, page turns and finished jobs. On a keyboard-opened wheel, Down reaches the ring choices. The newest job's buttons are in the tray menu.
- Choice groups in the tool windows and Settings move with the arrow keys and read as radio buttons.
- The tray and window icon follow the accent colour and the taskbar's light or dark mode; Start and the taskbar get light and dark versions of the mark.
- A native build for Windows on Arm, with its own FFmpeg and ONNX Runtime.
- Report a problem in Settings opens a filled-in GitHub issue in the browser; nothing is sent until you post it.
- Batch rename has Undo.
- The wheel fits every screen, scaling and monitor layout, and dark title bars use the setting early Windows 10 builds expect.
- Results can go into a subfolder or one folder instead of next to the original.
- Keep a tool's settings as a choice on its ring, like "To send" for a smaller video, from the tool's window.

### Removed

- Terminal is no longer on the Actions wheel.

## [0.1.1] - 2026-10-23

### Changed

- The website moved to [tapwheel.app](https://tapwheel.app); tapwheel.muxover.com sends visitors there.

### Fixed

- Compress on a 2.7 GB video ran out of memory and closed the app. Editors read files from disk in 4 MB pieces, off the UI thread.
- Tapping Alt during a drag lagged or showed Explorer's "Create link" while an editor was busy. The keyboard and mouse hooks run on their own thread and are set again if Windows drops them.
- Video editors waited up to 9.8 s for a filmstrip. They open after the file is read; Trim's filmstrip comes from 14 seeks, and Compress, Crop and Snapshot build none.
- Make GIF and Reverse held every frame in memory. Reverse works a piece at a time, and both check free memory first.
- The wheel was taller than the screen at 1366 by 768 and 125%. It shrinks to fit the work area.
- The wheel looked at every dragged file on disk on every frame. It reads them once.
- Memory stayed taken after the work was done: the background-removal model, loaded whenever a picture's Tools wheel opened, was never released, and neither was what a job or tool window left behind. After 30 quiet seconds TapWheel gives it back.
- A drop of thousands of files was planned on the UI thread. It's planned on the job's own thread.
- RTF, subtitle and INI inputs, damaged PSD and camera raw files, deep nesting and huge declared sizes could close the app. They fail on their card instead.
- Crashes the panic hook never sees are logged, with a minidump in `%LOCALAPPDATA%\TapWheel\crash`, and FFmpeg ends with the app.
- Edited photos kept the original thumbnail and Redact kept the location; Remove location missed GPS in XMP.
- Two jobs at once could share temporary files.
- Multi-member `.gz` and `.bz2` and multi-stream `.xz` files were cut short.
- JSON lost its key order and number formatting; CSV lost column order and duplicate columns.
- Animated GIFs lost their animation in the picture tools, and Combine dropped frames of mixed sizes.
- Text lost its encoding and BOM, and RTF ignored its code page.
- Indent picked the wrong step, Minify broke TOML strings and could damage CSV (Minify is gone for CSV), batch rename dropped `.tar.gz` and could replace a file that appeared after the preview, and stripping PNG and WebP details turned portraits sideways.
- Extract failed on one symbolic link or a name Windows can't hold. It skips symbolic links, unpacks hard links as copies and renames such names.
- Compress and Crop opened every video as a picture, and the tag editor opened sound files as photos: the app sent the tool windows a kind none of them recognised.
- Open sample files did nothing on many Store installs: Windows hides new AppData folders of Store apps from Explorer. The samples go to `Documents\TapWheel samples`, and Remove from this PC deletes them.
- Crop's size boxes rewrote a number while it was typed, Compress showed its video quality words in reverse, Enter on Cancel saved, and Save stayed on "Saving" after the app turned a save down.

## [0.1.0] - 2026-10-18

### Added

- Drag a file, tap Shift, Ctrl or Alt, and drop it on the Convert, Tools or Actions wheel. The wheel only opens while files are being dragged.
- Pictures: convert between 10 formats and read 48, including HEIC, JPEG XL, PSD and camera RAW. Crop, add a background (gradients, any colour or your own picture), remove the background and touch up the cut-out with brushes, adjust, annotate, redact, resize, compress to a quality or a target size, rotate, combine, trace to SVG, make icons, and see or remove hidden photo details.
- Video: convert, trim with several cuts, compress to a quality or a size like 25 MB, make GIFs, crop, resize, change speed, save frames (or pick the exact one), mute, save the sound, rotate, reverse and join.
- Audio: convert, trim, fade, even out, compress, change volume, speed and channels, make ringtones, remove silence, reverse, join and edit tags.
- Icons: Windows and macOS icons, favicon packs, PNG sets, and icons pulled out of programs.
- Text and code: Markdown and HTML, read aloud, UTF-8, clean up, sort, dedupe, change case, line endings, split and merge, count, format and minify for 30 file types, code shots and Copy as Markdown. Data, config and subtitle files only get the tools that keep their structure whole.
- Archives: extract, see what's inside and extract only what you tick, test, password-protect ZIP and 7Z with AES-256, split into parts, and convert between ZIP, 7Z and TAR kinds.
- Actions for any file: copy, copy the path, name, base name, extension, quoted path or file URL, rename one or many with a preview, move to a recent or chosen folder, duplicate, share, file info with the real type, float on a shelf, checksums, Properties, Open with, and a terminal for folders and code.
- Tools with choices show them on a ring around the slice, and tools that need setting up open their own window with a live preview.
- More than eight slices turn into pages; rest on More, press Tab, or roll the mouse wheel.
- Background removal runs on the PC with a bundled model.
- Compact progress cards that expand on hover, with sizes before and after, Open, Show, Undo, Cancel and Retry. Jobs wait their turn, check the file and free space first, and leave nothing behind when cancelled or interrupted.
- Undo only removes a file TapWheel made, and only if it hasn't changed since.
- A local error log, so problems can be found without guessing.
- Settings with an animated guide, keys, defaults for every tool, start with Windows, favourites first and pause.
- Windows 10 and 11 looks that follow light and dark mode, the accent colour and high contrast.
- Portable zip with a right-click entry, and a Microsoft Store package.

[Unreleased]: https://github.com/muxover/tapwheel/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/muxover/tapwheel/releases/tag/v0.1.1
[0.1.0]: https://github.com/muxover/tapwheel/releases/tag/v0.1.0
