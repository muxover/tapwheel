# Contributing

## Getting started

Install Rust (stable) on Windows 10 or 11, clone the repo, then fetch the converter and build:

```powershell
./engines/fetch.ps1
cargo build --release
target\release\tapwheel.exe
```

The core and UI crates also build and test on Linux and macOS; the app itself only runs on Windows.

## Running tests

```powershell
$env:TAPWHEEL_FFMPEG = "$PWD\target\engines\ffmpeg.exe"
cargo test --workspace
```

Without `TAPWHEEL_FFMPEG` the tests that run real conversions are skipped. To look at every screen without Windows, render them to PNG:

```bash
cargo run -p tapwheel-ui --example preview -- out/ <regular font> <semibold font>
```

## Code style

`cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` must pass.

## Submitting changes

Open an issue first for anything bigger than a small fix. Branch from `main`, keep one change per pull request, and describe how you tested it on Windows.

## Reporting bugs

Include the TapWheel version, Windows version (10 or 11, and the build), the file type you dragged, what you expected and what happened.
