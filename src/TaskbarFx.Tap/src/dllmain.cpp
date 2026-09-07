#include <Windows.h>

#include "appearance.hpp"
#include "tapsite.hpp"
#include "../../TaskbarFx.Shared/TaskbarFxTapProtocol.h"

namespace
{
    volatile LONG g_commandThreadStarted = 0;
    HANDLE g_stopEvent = nullptr;
    HANDLE g_applyEvent = nullptr;
    HWND g_msgWnd = nullptr;
    HANDLE g_commandThread = nullptr;

    constexpr UINT kApplyMsg = WM_APP + 7;

    bool InExplorer()
    {
        wchar_t name[MAX_PATH]{};
        if (GetModuleFileNameW(nullptr, name, MAX_PATH) == 0)
        {
            return false;
        }
        const wchar_t* base = wcsrchr(name, L'\\');
        base = base ? base + 1 : name;
        return _wcsicmp(base, L"explorer.exe") == 0;
    }

    LRESULT CALLBACK TapWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam)
    {
        if (msg == kApplyMsg)
        {
            AppearanceController::Instance().ApplyPendingFromShm();
            return 0;
        }
        return DefWindowProcW(hwnd, msg, wParam, lParam);
    }

    HWND CreateMessageWindow()
    {
        WNDCLASSEXW wc{};
        wc.cbSize = sizeof(wc);
        wc.lpfnWndProc = TapWndProc;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.lpszClassName = L"Suite.TaskbarFx.Tap.Msg";
        RegisterClassExW(&wc);
        return CreateWindowExW(
            0,
            wc.lpszClassName,
            L"Suite.TaskbarFx.Tap",
            0,
            0, 0, 0, 0,
            HWND_MESSAGE,
            nullptr,
            wc.hInstance,
            nullptr);
    }

    DWORD WINAPI CommandLoop(LPVOID)
    {
        if (g_applyEvent == nullptr)
        {
            g_applyEvent = OpenEventW(SYNCHRONIZE, FALSE, kTaskbarFxTapApplyEventName);
        }
        if (g_stopEvent == nullptr)
        {
            g_stopEvent = OpenEventW(SYNCHRONIZE, FALSE, kTaskbarFxTapStopEventName);
        }
        if (g_applyEvent == nullptr || g_stopEvent == nullptr)
        {
            return 0;
        }

        HANDLE waits[2] = { g_applyEvent, g_stopEvent };
        if (g_msgWnd)
        {
            PostMessageW(g_msgWnd, kApplyMsg, 0, 0);
        }
        for (;;)
        {
            const DWORD which = WaitForMultipleObjects(2, waits, FALSE, INFINITE);
            if (which == WAIT_OBJECT_0 + 1 || which == WAIT_FAILED)
            {
                break;
            }
            if (g_msgWnd)
            {
                PostMessageW(g_msgWnd, kApplyMsg, 0, 0);
            }
        }
        return 0;
    }

    DWORD WINAPI MaybeInstall(LPVOID)
    {
        Sleep(300);
        if (!TAPSite::SiteReady())
        {
            TAPSite::Install(nullptr);
        }
        return 0;
    }
}

void TAPSite::StartPump()
{
    // Site is attached; "can Apply" is the ready event, signaled only after
    // VisualTreeWatcher::AdviseVisualTreeChange returns (same as TranslucentTB).
    AppearanceController::Instance().AttachShm();
    if (g_msgWnd == nullptr)
    {
        g_msgWnd = CreateMessageWindow();
    }
    if (InterlockedCompareExchange(&g_commandThreadStarted, 1, 0) == 0)
    {
        g_stopEvent = CreateEventW(nullptr, TRUE, FALSE, kTaskbarFxTapStopEventName);
        g_applyEvent = CreateEventW(nullptr, FALSE, FALSE, kTaskbarFxTapApplyEventName);
        g_commandThread = CreateThread(nullptr, 0, CommandLoop, nullptr, 0, nullptr);
    }
}

extern "C" LRESULT CALLBACK TaskbarFxTapCallWndProc(int code, WPARAM wParam, LPARAM lParam)
{
    return CallNextHookEx(nullptr, code, wParam, lParam);
}

BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID)
{
    switch (reason)
    {
    case DLL_PROCESS_ATTACH:
        if (InExplorer())
        {
            HANDLE thread = CreateThread(nullptr, 0, MaybeInstall, nullptr, 0, nullptr);
            if (thread)
            {
                CloseHandle(thread);
            }
        }
        break;
    case DLL_PROCESS_DETACH:
        if (g_stopEvent)
        {
            SetEvent(g_stopEvent);
        }
        break;
    }
    return TRUE;
}
