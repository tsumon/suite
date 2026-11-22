# Suite v0.2.0 (Rust Native Edition)

Pure native **Win32 + Direct2D / DirectWrite + DXGI + WinRT OCR** rewrite of **Suite** (`tsumon/suite`), replacing the .NET 10 / WPF / WinForms + `TaskbarFx.Native.dll` stack with a single native `Suite.exe` (~1.3 MB) alongside the C++/WinRT Explorer XAML Diagnostics helper `TaskbarFx.Tap.dll` (~98 KB).

## Architecture (7 Crates + C++ TAP DLL)

```text
rust/crates/
  suite-contracts/   # JSONC v4 settings schema + v1..v4 migrations, paths, Chinese UI copy, chrome math
  suite-platform/    # Single-instance mutex, HKCU Run/Personalize, TaskbarProbe/Embed, CrashGuard, logs
  suite-netspeed/    # GetIfTable2 IP Helper sampling, active adapter resolver, rate calculation
  suite-theme/       # Atomic AppsUseLightTheme + SystemUsesLightTheme toggle with immediate local hook
  suite-taskbarfx/   # Absorbs TaskbarFx.Native: Win10 SWCA + Win11 shared-memory IPC to TaskbarFx.Tap.dll
  suite-capture/     # DXGI Desktop Duplication + GDI BitBlt grabbers, DPI cropper, pixel draw/mosaic, PNG, OCR, scroll
  suite-app/         # Native Win32 host (Suite.exe): tray, hotkeys, D2D NetSpeed/Pins, Fluent v2.1 Settings, History
```

Keeping `src/TaskbarFx.Tap/` in C++/WinRT preserves Explorer's `IXamlDiagnostics` / `ITapSite` COM activation ABI while removing the .NET CLR, WPF, WinForms, and `TaskbarFx.Native.dll`.

## Highlights in v0.2.0

- **Win11 Fluent Settings v2.1**: Per-Monitor DPI scaling, DWM dark titlebar & rounded corners, 208 DIP painted nav rail with pill & accent indicator, hairline section dividers, owner-drawn buttons & color swatch previews, quick-action shortcuts, live D2D taskbar chip preview strip, native `IFileOpenDialog` folder picker, auto-adapter selection, editable pin click-through hotkey, history cap dropdown, and direct release download CTA.
- **Downsampled History Thumbnails & Dark Mode**: Aspect-preserving thumbnails keep memory footprint minimal while supporting system theme sync and dimensions metadata.
- **Live Scroll-Capture Progress**: Dark floating status pill updates stitched pixel height in real time (`WM_APP_SCROLL_PROGRESS`).
- **Capture Overlay Hotkeys**: `Ctrl+C` (copy), `Ctrl+S` (save), `Ctrl+T` (pin), `Ctrl+O` (OCR), `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` (undo/redo), `1`–`6` (tool toggle), and `C` in selection phase for quick HEX color picking.

## Prerequisites

- Windows 10 / Windows 11 (x64)
- Rust toolchain (`rustc` / `cargo`, MSVC x64 target)
- Visual Studio 2022 Build Tools (for `TaskbarFx.Tap.dll`)
- Optional: Inno Setup 6 (`ISCC.exe`) for building `Suite-Setup-0.2.0.exe`

## Build & Test

Run clippy and unit tests from `rust/`:

```powershell
cargo clippy --workspace --all-targets --offline
cargo test --workspace --offline
```

Build and package portable + installer outputs into `dist-rs/`:

```powershell
powershell -ExecutionPolicy Bypass -File ..\scripts\build-rust.ps1
```

Launch the Inno Setup installer locally to upgrade in-place over 0.1.x:

```powershell
powershell -ExecutionPolicy Bypass -File ..\scripts\install-local.ps1
```

## Isolated Smoke Check

`Suite.exe --smoke-check` boots the controller in an isolated mode, initializes Direct2D/DirectWrite, probes WinRT OCR and taskbar state, samples live adapter throughput, emits a single JSON telemetry line on stdout, and exits cleanly with `cleanExit: true`:

```powershell
$env:SUITE_DATA_DIR = "$env:TEMP\suite-rs-smoke"
$env:SUITE_MUTEX_NAME = "Local\Suite.Rs.SmokeTest"
& ..\dist-rs\Suite.exe --smoke-check | Out-String
```

## Compatibility & Licensing

- Reads and writes existing `%LOCALAPPDATA%\Suite\settings.json` (schema v4, including JSONC comments and v1..v4 migrations), `crash-guard.json`, `logs/`, and `Screenshots/` capture/history folders.
- Licensed under **GPL-3.0**. See `LICENSE` and `NOTICE`.
