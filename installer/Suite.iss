; Suite - Inno Setup 6 (v0.3.0 Rust Native)
; GPL-3.0 - see LICENSE / NOTICE in the repo root.

#define MyAppName "Suite"
#define MyAppVersion "0.3.0"
#define MyAppPublisher "tsumon"
#define MyAppExeName "Suite.exe"
#ifndef StageDir
  #define StageDir "..\dist-rs"
#endif

[Setup]
AppId={{A8C3E2F1-5B7D-4E9A-9C1F-2D4E6F80A1B3}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL=https://github.com/tsumon/suite
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
InfoBeforeFile=..\NOTICE
OutputDir={#StageDir}
OutputBaseFilename=Suite-Setup-{#MyAppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
PrivilegesRequired=admin
AppMutex=Local\Suite.App.SingleInstance
CloseApplications=force
RestartApplications=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked
Name: "autostart"; Description: "Start Suite at logon"; GroupDescription: "Startup:"

[InstallDelete]
; Clean up legacy .NET 10 / C++ native sidecar binaries when upgrading from 0.1.x
Type: files; Name: "{app}\Suite.dll"
Type: files; Name: "{app}\Suite.*.dll"
Type: files; Name: "{app}\Suite.deps.json"
Type: files; Name: "{app}\Suite.runtimeconfig.json"
Type: files; Name: "{app}\TaskbarFx.Native.dll"
Type: files; Name: "{app}\System.*.dll"
Type: files; Name: "{app}\Microsoft.*.dll"
Type: files; Name: "{app}\WinRT.Runtime.dll"
Type: files; Name: "{app}\*.deps.json"
Type: files; Name: "{app}\*.runtimeconfig.json"

[Files]
Source: "{#StageDir}\Suite.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\TaskbarFx.Tap.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Suite"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch Suite"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True;
  if not WizardSilent() then
    MsgBox('Antivirus may block TaskbarFx.Tap.dll (taskbar effects). Add an exclusion if settings report a failure.', mbInformation, MB_OK);
end;
