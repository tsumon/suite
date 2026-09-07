#pragma once

// Host-facing TAP exports. The DLL is loaded into explorer.exe via
// InitializeXamlDiagnosticsEx (absolute path). No CLR.

#ifdef __cplusplus
extern "C" {
#endif

#include <Windows.h>

LRESULT CALLBACK TaskbarFxTapCallWndProc(int code, WPARAM wParam, LPARAM lParam);

#ifdef __cplusplus
}
#endif
