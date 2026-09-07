; Suite — Inno Setup 6 draft (NOT compiled on box / Linux).
; Adjust Source paths to your `dotnet publish` + Native/TAP output on Windows.
; GPL-3.0 — see LICENSE / NOTICE in the repo root.

#define MyAppName "Suite"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Joe / contributors"
#define MyAppExeName "Suite.exe"
; Example publish dir — change before compile:
#define PublishDir "..\src\Suite.App\bin\x64\Release\net10.0-windows10.0.19041.0\win-x64\publish"

[Setup]
AppId={{A8C3E2F1-5B7D-4E9A-9C1F-2D4E6F80A1B3}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
InfoBeforeFile=..\NOTICE
OutputDir=.
OutputBaseFilename=Suite-Setup-{#MyAppVersion}
Compression=lzma
SolidCompression=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加图标:"; Flags: unchecked
Name: "autostart"; Description: "登录时启动 Suite"; GroupDescription: "启动:"

[Files]
; Managed app (publish output). Excludes PDB noise.
Source: "{#PublishDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "*.pdb"
; Native / TAP must sit next to Suite.exe (override if publish already copied them).
; Source: "..\path\to\TaskbarFx.Native.dll"; DestDir: "{app}"; Flags: ignoreversion
; Source: "..\path\to\TaskbarFx.Tap.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\卸载 {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; Optional Run-at-login (HKCU). Mirrors Suite settings StartWithWindows.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Suite"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "启动 Suite"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True;
  MsgBox('杀软可能拦截 TaskbarFx.Tap.dll（任务栏效果）。若设置页提示被拦住，请加入排除项或改用便携目录。', mbInformation, MB_OK);
end;
