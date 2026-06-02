# Oxide Reader

A simple, lightweight PDF viewer for Windows built in Rust.
Dark theme by default. No installer required — just an `.exe` and one `.dll`.

![Dark themed PDF viewer with toolbar showing open, navigation, and zoom controls]

## Features

- Open PDF files via button, `Ctrl+O`, or drag-and-drop onto the window
- Page navigation with `◀ ▶` buttons or `←` `→` / `Page Up` / `Page Down` keys
- Zoom in/out with `+` `−` buttons or `Ctrl++` / `Ctrl+−` / `Ctrl+0` (reset)
- "Open with" support — associate `.pdf` files with Oxide Reader in Windows
- Soft drop-shadow page rendering on a deep navy dark background

## Building from source

### 1. Prerequisites

- [Rust](https://rustup.rs) (edition 2021, stable toolchain)

### 2. Download the PDF rendering library

Run the included script once to download `pdfium.dll` (Google's PDFium engine, ~7 MB):

```powershell
.\get_pdfium.ps1
```

### 3. Build

```powershell
cargo build --release
```

The output is in `target\release\`:

```
target\release\oxide-reader.exe   (~4.5 MB)
target\release\pdfium.dll         (~7 MB, auto-copied by build.rs)
```

## Distribution

Copy these two files to any folder — no installer or runtime dependencies needed:

```
oxide-reader.exe
pdfium.dll
```

## Keyboard shortcuts

| Action | Shortcut |
|---|---|
| Open file | `Ctrl+O` |
| Next page | `→` or `Page Down` |
| Previous page | `←` or `Page Up` |
| Zoom in | `Ctrl++` |
| Zoom out | `Ctrl+−` |
| Reset zoom | `Ctrl+0` |

## Tech stack

| Component | Crate |
|---|---|
| UI framework | [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) |
| PDF rendering | [pdfium-render](https://github.com/ajrcarey/pdfium-render) (wraps Google PDFium) |
| File dialogs | [rfd](https://github.com/PolyMeilex/rfd) |
