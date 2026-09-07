#include "TaskbarFxNative.h"
#include "taskbarfx_internal.h"
#include "../../TaskbarFx.Shared/TaskbarFxTapProtocol.h"

#include <Windows.h>

#include <string>
#include <cstdio>

// Win11 XAML path. Loads TaskbarFx.Tap.dll into explorer via the documented
// InitializeXamlDiagnosticsEx TAP entry (off-label target PID = explorer).
// TAP implementation is adapted from TranslucentTB ExplorerTAP (GPL-3.0).
// Absolute path only (security/AUDIT.md F-04). Never TerminateProcess(explorer).

extern HMODULE g_taskbarFxNativeModule;

namespace
{
    using InitializeXamlDiagnosticsExFn = HRESULT(WINAPI*)(LPCWSTR, DWORD, LPCWSTR, LPCWSTR, CLSID, LPCWSTR);

    HANDLE g_mapping = nullptr;
    TaskbarFxTapShm* g_shm = nullptr;
    HANDLE g_ready = nullptr;
    HANDLE g_apply = nullptr;
    HMODULE g_tapModule = nullptr;
    HHOOK g_hook = nullptr;
    bool g_injected = false;
    DWORD g_explorerPid = 0;

    std::wstring NativeDirectory()
    {
        wchar_t path[MAX_PATH]{};
        HMODULE module = g_taskbarFxNativeModule;
        if (module == nullptr)
        {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                reinterpret_cast<LPCWSTR>(&NativeDirectory),
                &module);
        }
        if (module == nullptr || GetModuleFileNameW(module, path, MAX_PATH) == 0)
        {
            return {};
        }
        std::wstring full(path);
        const size_t slash = full.find_last_of(L"\\/");
        if (slash == std::wstring::npos)
        {
            return {};
        }
        return full.substr(0, slash);
    }

    std::wstring TapDllPath()
    {
        std::wstring dir = NativeDirectory();
        if (dir.empty())
        {
            return {};
        }
        return dir + L"\\TaskbarFx.Tap.dll";
    }

    bool FileExists(const std::wstring& path)
    {
        const DWORD attrs = GetFileAttributesW(path.c_str());
        return attrs != INVALID_FILE_ATTRIBUTES && (attrs & FILE_ATTRIBUTE_DIRECTORY) == 0;
    }

    DWORD ExplorerPidFromTray()
    {
        HWND tray = FindWindowW(L"Shell_TrayWnd", nullptr);
        if (tray == nullptr)
        {
            return 0;
        }
        DWORD pid = 0;
        GetWindowThreadProcessId(tray, &pid);
        return pid;
    }

    DWORD ExplorerThreadFromTray()
    {
        HWND tray = FindWindowW(L"Shell_TrayWnd", nullptr);
        if (tray == nullptr)
        {
            return 0;
        }
        DWORD pid = 0;
        return GetWindowThreadProcessId(tray, &pid);
    }

    bool EnsureShm()
    {
        if (g_shm)
        {
            return true;
        }

        g_ready = CreateEventW(nullptr, TRUE, FALSE, kTaskbarFxTapReadyEventName);
        g_apply = CreateEventW(nullptr, FALSE, FALSE, kTaskbarFxTapApplyEventName);
        g_mapping = CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            nullptr,
            PAGE_READWRITE,
            0,
            sizeof(TaskbarFxTapShm),
            kTaskbarFxTapShmName);
        if (g_mapping == nullptr || g_ready == nullptr || g_apply == nullptr)
        {
            return false;
        }

        g_shm = static_cast<TaskbarFxTapShm*>(MapViewOfFile(g_mapping, FILE_MAP_ALL_ACCESS, 0, 0, sizeof(TaskbarFxTapShm)));
        if (g_shm == nullptr)
        {
            return false;
        }

        if (g_shm->magic != kTaskbarFxTapMagic)
        {
            ZeroMemory(g_shm, sizeof(TaskbarFxTapShm));
            g_shm->magic = kTaskbarFxTapMagic;
            g_shm->version = kTaskbarFxTapVersion;
        }
        g_shm->hostPid = GetCurrentProcessId();
        return true;
    }

    bool CallIxde(const std::wstring& tapPath, DWORD pid)
    {
        HMODULE wux = LoadLibraryExW(L"Windows.UI.Xaml.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
        if (wux == nullptr)
        {
            SetLast(L"Windows.UI.Xaml.dll is not available.");
            return false;
        }

        const auto ixde = reinterpret_cast<InitializeXamlDiagnosticsExFn>(
            GetProcAddress(wux, "InitializeXamlDiagnosticsEx"));
        if (ixde == nullptr)
        {
            SetLast(L"InitializeXamlDiagnosticsEx is not exported.");
            return false;
        }

        HRESULT hr = E_FAIL;
        for (int attempt = 1; attempt <= 5; ++attempt)
        {
            wchar_t conn[64];
            swprintf_s(conn, L"SuiteTaskbarFxDiag%d", attempt);
            hr = ixde(conn, pid, nullptr, tapPath.c_str(), CLSID_TaskbarFxTapSite, nullptr);
            if (SUCCEEDED(hr))
            {
                return true;
            }
            Sleep(300);
        }

        wchar_t msg[192];
        swprintf_s(msg, L"InitializeXamlDiagnosticsEx failed hr=0x%08X.", static_cast<unsigned>(hr));
        SetLast(msg);
        {
            wchar_t tmp[MAX_PATH]{};
            GetTempPathW(MAX_PATH, tmp);
            std::wstring path(tmp);
            path += L"suite-ixde.log";
            FILE* fp = nullptr;
            if (_wfopen_s(&fp, path.c_str(), L"wb") == 0 && fp)
            {
                fprintf(fp, "hr=0x%08X pid=%lu\n", static_cast<unsigned>(hr), static_cast<unsigned long>(pid));
                fclose(fp);
            }
        }
        return false;
    }

    bool WaitReady(DWORD timeoutMs)
    {
        if (g_ready == nullptr)
        {
            return false;
        }
        return WaitForSingleObject(g_ready, timeoutMs) == WAIT_OBJECT_0;
    }

    bool HookFallback(const std::wstring& tapPath)
    {
        if (g_tapModule == nullptr)
        {
            g_tapModule = LoadLibraryExW(
                tapPath.c_str(),
                nullptr,
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
        }
        if (g_tapModule == nullptr)
        {
            SetLast(L"TaskbarFx.Tap.dll could not be loaded.");
            return false;
        }

        auto proc = reinterpret_cast<HOOKPROC>(GetProcAddress(g_tapModule, "TaskbarFxTapCallWndProc"));
        if (proc == nullptr)
        {
            SetLast(L"TaskbarFx.Tap.dll is missing TaskbarFxTapCallWndProc.");
            return false;
        }

        const DWORD tid = ExplorerThreadFromTray();
        if (tid == 0)
        {
            SetLast(L"No Shell_TrayWnd. Taskbar is not available.");
            return false;
        }

        g_hook = SetWindowsHookExW(WH_CALLWNDPROC, proc, g_tapModule, tid);
        if (g_hook == nullptr)
        {
            SetLast(L"Could not load the Win11 taskbar helper into Explorer.");
            return false;
        }

        HWND tray = FindWindowW(L"Shell_TrayWnd", nullptr);
        if (tray)
        {
            SendMessageTimeoutW(tray, WM_NULL, 0, 0, SMTO_ABORTIFHUNG, 1000, nullptr);
        }

        const bool ready = WaitReady(35000);
        if (g_hook)
        {
            UnhookWindowsHookEx(g_hook);
            g_hook = nullptr;
        }
        return ready;
    }

    bool PingTapPump(DWORD timeoutMs)
    {
        if (g_shm == nullptr || g_apply == nullptr)
        {
            return false;
        }
        const uint32_t next = g_shm->seq + 1;
        g_shm->mode = TaskbarFxTapModeNormal;
        g_shm->argb = 0;
        g_shm->result = TaskbarFxTapResultFailed;
        g_shm->seq = next;
        SetEvent(g_apply);
        const ULONGLONG start = GetTickCount64();
        while (GetTickCount64() - start < timeoutMs)
        {
            if (g_shm->ack == next)
            {
                return true;
            }
            Sleep(20);
        }
        return false;
    }

    int EnsureInjected()
    {
        const DWORD pid = ExplorerPidFromTray();
        if (pid == 0)
        {
            SetLast(L"No Shell_TrayWnd. Taskbar is not available.");
            return TaskbarFxErrNoTray;
        }

        if (!EnsureShm())
        {
            SetLast(L"Could not create the Win11 taskbar helper channel.");
            return TaskbarFxErrTapInit;
        }

        // Prefer a live TAP pump over the ready event (ready can be stale/false).
        if (g_injected && g_explorerPid == pid && PingTapPump(400))
        {
            return TaskbarFxOk;
        }
        if (g_shm->explorerPid == pid && PingTapPump(400))
        {
            g_injected = true;
            g_explorerPid = pid;
            g_shm->ready = 1;
            return TaskbarFxOk;
        }

        g_injected = false;
        g_explorerPid = 0;
        g_shm->ready = 0;

        const std::wstring tapPath = TapDllPath();
        if (tapPath.empty() || !FileExists(tapPath))
        {
            SetLast(L"TaskbarFx.Tap.dll is missing. Build TaskbarFx.Tap in Visual Studio.");
            return TaskbarFxErrTapInit;
        }

        // Fresh inject: clear ready so WaitReady waits for Advise again.
        ResetEvent(g_ready);

        // Prefer hook-first (TranslucentTB): load TAP into explorer, then IXDE on a fresh
        // thread inside explorer. Remote IXDE from Suite is a fallback.
        if (!HookFallback(tapPath))
        {
            const bool ixdeOk = CallIxde(tapPath, pid);
            if (!WaitReady(ixdeOk ? 35000 : 2000))
            {
                SetLast(L"Win11 taskbar helper did not start. Antivirus may have blocked it.");
                return TaskbarFxErrTapInit;
            }
        }

        g_injected = true;
        g_explorerPid = pid;
        g_shm->ready = 1;
        g_shm->explorerPid = pid;

        // Confirm the pump is actually alive.
        if (!PingTapPump(1000))
        {
            SetLast(L"Win11 taskbar helper started but did not respond.");
            return TaskbarFxErrTapInit;
        }
        return TaskbarFxOk;
    }
}

int ApplyWin11Xaml(int mode, unsigned int argb)
{
    const int inject = EnsureInjected();
    if (inject != TaskbarFxOk)
    {
        return inject;
    }
    if (g_shm == nullptr || g_apply == nullptr)
    {
        SetLast(L"Win11 taskbar helper channel is missing.");
        return TaskbarFxErrTapInit;
    }

    const uint32_t next = g_shm->seq + 1;
    g_shm->mode = mode;
    g_shm->argb = argb;
    g_shm->result = TaskbarFxTapResultPendingFills;
    g_shm->fillCount = 0;
    g_shm->seq = next;
    SetEvent(g_apply);

    const ULONGLONG start = GetTickCount64();
    constexpr ULONGLONG kPendingWaitMs = 8000;
    constexpr DWORD kPendingRetryMs = 200;
    while (GetTickCount64() - start < kPendingWaitMs)
    {
        if (g_shm->ack == next && g_shm->result != TaskbarFxTapResultPendingFills)
        {
            break;
        }
        if (g_shm->ack == next && g_shm->result == TaskbarFxTapResultPendingFills)
        {
            SetEvent(g_apply);
        }
        Sleep(kPendingRetryMs);
    }

    if (g_shm->ack != next || g_shm->result == TaskbarFxTapResultPendingFills)
    {
        SetLast(L"暂时找不到任务栏背景，请再点一次应用。");
        return TaskbarFxErrCallFailed;
    }

    if (g_shm->result == TaskbarFxTapResultFailed)
    {
        SetLast(L"Win11 taskbar appearance could not be applied.");
        return TaskbarFxErrCallFailed;
    }

    if (mode == TaskbarFxModeNormal)
    {
        SetLast(L"Restored system default (did not end explorer).");
    }
    else if (g_shm->result == TaskbarFxTapResultAcrylicFallback)
    {
        SetLast(L"Applied Win11 XAML appearance. Acrylic fell back to a solid tint.");
    }
    else
    {
        SetLast(L"Applied Win11 XAML appearance.");
    }
    return TaskbarFxOk;
}
