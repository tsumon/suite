# Suite (Rust Native Edition)

Pure native Win32 + Direct2D / DirectWrite + WinRT OCR rewrite of **Suite** (`tsumon/suite` v0.1.8 baseline), replacing the .NET 10 / WPF / WinForms + `TaskbarFx.Native.dll` stack with a single native `Suite.exe` plus the small C++/WinRT XAML Diagnostics injection module `TaskbarFx.Tap.dll`.

## Architecture (7 Crates + C++ TAP DLL)

```text
rust/crates/
  suite-contracts/   # JSONC v4 settings schema + v1..v4 migrations, paths, Chinese UI copy, chrome math
  suite-platform/    # Single-instance mutex, HKCU Run/Personalize, TaskbarProbe/Embed, CrashGuard, logs
  suite-netspeed/    # GetIfTable2 IP Helper sampling, active adapter resolver, rate calculation
  suite-theme/       # Atomic AppsUseLightTheme + SystemUsesLightTheme toggle with immediate local hook
  suite-taskbarfx/   # Absorbs TaskbarFx.Native: Win10 SWCA + Win11 shared-memory IPC to TaskbarFx.Tap.dll
  suite-capture/     # DXGI Desktop Duplication + GDI BitBlt grabbers, DPI cropper, pixel draw/mosaic, PNG, OCR, scroll
  suite-app/         # Native Win32 host (Suite.exe): tray, hotkeys, D2D NetSpeed/Pins, capture/annotate overlay, settings/history
```

Keeping `src/TaskbarFx.Tap/` in C++/WinRT preserves Explorer's `IXamlDiagnostics` / `ITapSite` COM activation ABI while eliminating the .NET CLR, WPF, WinForms, and `TaskbarFx.Native.dll`.

## Prerequisites

- Windows 10 / Windows 11 (x64)
- Rust toolchain (`rustc` / `cargo`, MSVC x64 target)
- Visual Studio 2022 Build Tools (for rebuilding `TaskbarFx.Tap.dll` if needed)

## Build & Test

Run tests and clippy from `rust/`:

```powershell
cargo clippy --workspace --all-targets --offline
cargo test --workspace --offline
```

Build and stage the portable bundle into `dist-rs/`:

```powershell
powershell -ExecutionPolicy Bypass -File ..\scripts\build-rust.ps1
```

## Isolated Smoke Check

`Suite.exe --smoke-check` boots the controller in an isolated mode, initializes Direct2D/DirectWrite, probes WinRT OCR and taskbar state, samples live adapter throughput, emits a single JSON telemetry line on stdout, and exits cleanly with `cleanExit: true`:

```powershell
$env:SUITE_DATA_DIR = "$env:TEMP\suite-rs-smoke"
$env:SUITE_MUTEX_NAME = "Local\Suite.Rs.SmokeTest"
& .\dist-rs\Suite.exe --smoke-check | Out-String
```

## Compatibility & Licensing

- Reads and writes existing `%LOCALAPPDATA%\Suite\settings.json` (schema v4, including JSONC comments and v1..v4 migrations), `crash-guard.json`, `logs/`, and `Screenshots/` capture/history folders.
- Licensed under **GPL-3.0**. See `LICENSE` and `NOTICE` (including TranslucentTB attribution for `TaskbarFx.Tap.dll` and Windows 11 taskbar integration).
