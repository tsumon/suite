# Suite — Rust Native Build Script (Win32 + Direct2D/DirectWrite + WinRT OCR)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$RustDir = Join-Path $Root "rust"
$DistDir = Join-Path $Root "dist-rs"
$TapVcxproj = Join-Path $Root "src\TaskbarFx.Tap\TaskbarFx.Tap.vcxproj"
$TapDll = Join-Path $Root "src\TaskbarFx.Tap\bin\x64\Release\TaskbarFx.Tap.dll"

function Find-MsBuild {
    $cmd = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $found = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find "MSBuild\**\Bin\MSBuild.exe" 2>$null | Select-Object -First 1
        if ($found -and (Test-Path $found)) { return $found }
    }
    return $null
}

Push-Location $Root
try {
    $msbuild = Find-MsBuild
    if ($msbuild -and (Test-Path $TapVcxproj)) {
        Write-Host "==> Building C++/WinRT TaskbarFx.Tap.dll (Release|x64)"
        & $msbuild $TapVcxproj /m /p:Configuration=Release /p:Platform=x64 /verbosity:minimal
        if ($LASTEXITCODE -ne 0) { throw "MSBuild failed for TaskbarFx.Tap.vcxproj" }
    } elseif (-not (Test-Path $TapDll)) {
        throw "MSBuild not found and prebuilt TaskbarFx.Tap.dll missing at $TapDll"
    } else {
        Write-Host "==> Reusing prebuilt TaskbarFx.Tap.dll ($TapDll)"
    }

    Write-Host "==> Building Rust release workspace"
    Push-Location $RustDir
    try {
        cargo build --release --offline
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Offline build failed, retrying with cargo cache/network..."
            cargo build --release
            if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
        }
    } finally {
        Pop-Location
    }

    Write-Host "==> Staging portable dist-rs\"
    if (-not (Test-Path $DistDir)) {
        New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
    }
    $ExeSrc = Join-Path $RustDir "target\release\Suite.exe"
    if (-not (Test-Path $ExeSrc)) { throw "Missing release binary: $ExeSrc" }

    Copy-Item -Force $ExeSrc (Join-Path $DistDir "Suite.exe")
    Copy-Item -Force $TapDll (Join-Path $DistDir "TaskbarFx.Tap.dll")
    Copy-Item -Force (Join-Path $Root "LICENSE") (Join-Path $DistDir "LICENSE")
    Copy-Item -Force (Join-Path $Root "NOTICE") (Join-Path $DistDir "NOTICE")

    Write-Host "==> Portable output ready at $DistDir"
    Get-ChildItem $DistDir | Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize
} finally {
    Pop-Location
}
