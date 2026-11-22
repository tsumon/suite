# Suite — Local In-Place Upgrade Script (v0.1.x / v0.2.x -> v0.3.0 Rust Native)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$SetupExe = Join-Path $Root "dist-rs\Suite-Setup-0.3.0.exe"

if (-not (Test-Path $SetupExe)) {
    Write-Host "==> Installer not found; building v0.3.0 first..."
    & (Join-Path $PSScriptRoot "build-rust.ps1")
}

if (-not (Test-Path $SetupExe)) {
    throw "Missing installer at $SetupExe"
}

Write-Host "==> Launching Inno Setup upgrade installer ($SetupExe)"
Write-Host "    (Prompts for UAC elevation if needed, closes running Suite cleanly,"
Write-Host "     removes legacy .NET DLLs if present, and starts Suite v0.3.0 Rust Native.)"
Start-Process -FilePath $SetupExe
