#pragma once

#ifdef TASKBARFX_NATIVE_EXPORTS
#define TASKBARFX_API __declspec(dllexport)
#else
#define TASKBARFX_API __declspec(dllimport)
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum TaskbarFxKind
{
    TaskbarFxKindNotFound = 0,
    TaskbarFxKindClassicWin32 = 1,
    TaskbarFxKindModernXaml = 2
};

enum TaskbarFxMode
{
    TaskbarFxModeNormal = 0,
    TaskbarFxModeOpaque = 1,
    TaskbarFxModeClear = 2,
    TaskbarFxModeAcrylic = 3
};

enum TaskbarFxResult
{
    TaskbarFxOk = 0,
    TaskbarFxErrMissingExport = 1,
    TaskbarFxErrNoTray = 2,
    TaskbarFxErrModernXaml = 3,
    TaskbarFxErrCallFailed = 4,
    TaskbarFxErrBadMode = 5,
    TaskbarFxErrTapInit = 6
};

// Apply appearance. Classic trays use SWCA. Modern XAML trays inject TaskbarFx.Tap.dll
// via InitializeXamlDiagnosticsEx (absolute path). Does NOT kill explorer.exe.
TASKBARFX_API int __stdcall TaskbarFxApply(int mode, unsigned int argb);

// Same as Apply(normal). Restore default composition / original XAML fill.
// Never TerminateProcess(explorer).
TASKBARFX_API int __stdcall TaskbarFxReset(void);

// 0 = no tray, 1 = classic Win32, 2 = modern XAML (Win11 TAP path).
TASKBARFX_API int __stdcall TaskbarFxProbeKind(void);

TASKBARFX_API int __stdcall TaskbarFxLastMessage(wchar_t* buffer, int cch);

#ifdef __cplusplus
}
#endif
