# Suite smoke — INTERACTION-P2 §10. Run on Windows with .NET SDK.
# Does not claim UI green from Debian.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "src\Suite.sln"))) {
  Write-Host "SMOKE FAIL: repo root not found"
  exit 1
}

function Step([string]$Name, [scriptblock]$Body) {
  Write-Host "==> $Name"
  & $Body
  if ($LASTEXITCODE -ne 0) {
    Write-Host "SMOKE FAIL: $Name"
    exit 1
  }
}

Push-Location $Root
try {
  Step "restore" { dotnet restore src/Suite.sln }
  Step "build Contracts" { dotnet build src/Suite.Contracts/Suite.Contracts.csproj -c Release --no-restore }
  Step "test Contracts" { dotnet test tests/Suite.Contracts.Tests/Suite.Contracts.Tests.csproj -c Release --no-restore --verbosity quiet }
  Step "test Capture" { dotnet test tests/Suite.Capture.Tests/Suite.Capture.Tests.csproj -c Release --no-restore --verbosity quiet }
  Step "test NetSpeed" { dotnet test tests/Suite.NetSpeed.Tests/Suite.NetSpeed.Tests.csproj -c Release --no-restore --verbosity quiet }
  Step "build App" { dotnet build src/Suite.App/Suite.App.csproj -c Release --no-restore }
  Write-Host "SMOKE PASS"
  exit 0
}
finally {
  Pop-Location
}
