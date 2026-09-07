#include "TaskbarFxNative.h"
#include "taskbarfx_internal.h"

#include <Windows.h>

#include <cstring>
#include <vector>
#include <string>

// -----------------------------------------------------------------------------
// Undocumented user32!SetWindowCompositionAttribute
//
// This is NOT a documented Win32 API. There is no official reference page.
// Windows updates have broken and later restored similar tricks (public
// discussion: Microsoft Q&A 1720291; WindowsAppSDK discussion 2711).
//
// Layout below is the community-described accent policy (WCA_ACCENT_POLICY = 19).
// Implemented here from public discussion — not copied from GPL headers or TAP.
//
// Classic Shell_TrayWnd trees (no XAML island) use SWCA.
// Modern XAML taskbars (DesktopWindowContentBridge) go to ApplyWin11Xaml / TAP.
//
// Restore = ACCENT_DISABLED on the tray HWND, or original XAML Fill.
// Do NOT TerminateProcess(explorer) as a default recovery path.
// -----------------------------------------------------------------------------

namespace
{
    constexpr unsigned int kAccentPolicyAttribute = 19;

    enum AccentState : unsigned int
    {
        AccentDisabled = 0,
        AccentGradient = 1,
        AccentTransparentGradient = 2,
        AccentBlurBehind = 3,     // v1 Out (Blur)
        AccentAcrylicBlurBehind = 4
    };

    struct AccentPolicy
    {
        unsigned int state;
        unsigned int flags;
        unsigned int gradientAbgr;
        unsigned int animationId;
    };

    struct CompositionAttributeData
    {
        unsigned int attribute;
        void* data;
        unsigned long dataSize;
    };

    using SetWindowCompositionAttributeFn = BOOL(WINAPI*)(HWND, CompositionAttributeData*);

    wchar_t g_lastMessage[512] = L"";
}

void SetLast(const wchar_t* text)
{
    wcsncpy_s(g_lastMessage, text, _TRUNCATE);
}

namespace
{

    unsigned int ArgbToAbgr(unsigned int argb)
    {
        const unsigned int a = (argb >> 24) & 0xFFu;
        const unsigned int r = (argb >> 16) & 0xFFu;
        const unsigned int g = (argb >> 8) & 0xFFu;
        const unsigned int b = argb & 0xFFu;
        return (a << 24) | (b << 16) | (g << 8) | r;
    }

    SetWindowCompositionAttributeFn LoadSwca()
    {
        HMODULE user32 = GetModuleHandleW(L"user32.dll");
        if (user32 == nullptr)
        {
            user32 = LoadLibraryW(L"user32.dll");
        }
        if (user32 == nullptr)
        {
            return nullptr;
        }

        FARPROC proc = GetProcAddress(user32, "SetWindowCompositionAttribute");
        if (proc == nullptr)
        {
            return nullptr;
        }

        SetWindowCompositionAttributeFn fn = nullptr;
        static_assert(sizeof(fn) == sizeof(proc), "FARPROC size");
        memcpy(&fn, &proc, sizeof(fn));
        return fn;
    }

    bool HasXamlBridge(HWND tray)
    {
        if (tray == nullptr)
        {
            return false;
        }

        // Modern Win11 taskbar hosts a XAML island under Shell_TrayWnd.
        HWND bridge = FindWindowExW(
            tray,
            nullptr,
            L"Windows.UI.Composition.DesktopWindowContentBridge",
            nullptr);
        return bridge != nullptr;
    }

    std::wstring ClassNameOf(HWND hwnd)
    {
        wchar_t name[256];
        name[0] = 0;
        GetClassNameW(hwnd, name, 256);
        return name;
    }

    struct TrayEnumState
    {
        std::vector<HWND>* list;
    };

    BOOL CALLBACK EnumSecondary(HWND hwnd, LPARAM lParam)
    {
        auto* state = reinterpret_cast<TrayEnumState*>(lParam);
        if (ClassNameOf(hwnd) == L"Shell_SecondaryTrayWnd")
        {
            state->list->push_back(hwnd);
        }
        return TRUE;
    }

    std::vector<HWND> CollectTrays()
    {
        std::vector<HWND> trays;
        HWND primary = FindWindowW(L"Shell_TrayWnd", nullptr);
        if (primary != nullptr)
        {
            trays.push_back(primary);
        }

        TrayEnumState state{ &trays };
        EnumWindows(EnumSecondary, reinterpret_cast<LPARAM>(&state));
        return trays;
    }

    int ProbeKind()
    {
        std::vector<HWND> trays = CollectTrays();
        if (trays.empty())
        {
            SetLast(L"No Shell_TrayWnd. Taskbar is not available.");
            return TaskbarFxKindNotFound;
        }

        for (HWND hwnd : trays)
        {
            if (HasXamlBridge(hwnd))
            {
                SetLast(L"Modern XAML taskbar.");
                return TaskbarFxKindModernXaml;
            }
        }

        SetLast(L"Classic Win32 taskbar.");
        return TaskbarFxKindClassicWin32;
    }

    bool FillPolicy(int mode, unsigned int argb, AccentPolicy* policy)
    {
        policy->flags = 0x2; // draw the supplied color
        policy->animationId = 0;
        policy->gradientAbgr = ArgbToAbgr(argb);

        switch (mode)
        {
        case TaskbarFxModeNormal:
            policy->state = AccentDisabled;
            policy->flags = 0;
            policy->gradientAbgr = 0;
            return true;
        case TaskbarFxModeOpaque:
            policy->state = AccentGradient;
            policy->gradientAbgr = ArgbToAbgr(0xFF000000u | (argb & 0x00FFFFFFu));
            return true;
        case TaskbarFxModeClear:
            policy->state = AccentTransparentGradient;
            policy->gradientAbgr = ArgbToAbgr(argb & 0x00FFFFFFu); // alpha 0
            return true;
        case TaskbarFxModeAcrylic:
            policy->state = AccentAcrylicBlurBehind;
            return true;
        default:
            return false;
        }
    }

    int ApplyToHwnd(SetWindowCompositionAttributeFn fn, HWND hwnd, const AccentPolicy& policy)
    {
        AccentPolicy local = policy;
        CompositionAttributeData data{};
        data.attribute = kAccentPolicyAttribute;
        data.data = &local;
        data.dataSize = static_cast<unsigned long>(sizeof(local));
        if (!fn(hwnd, &data))
        {
            return TaskbarFxErrCallFailed;
        }
        return TaskbarFxOk;
    }
}

int __stdcall TaskbarFxProbeKind(void)
{
    return ProbeKind();
}

int __stdcall TaskbarFxApply(int mode, unsigned int argb)
{
    if (mode < TaskbarFxModeNormal || mode > TaskbarFxModeAcrylic)
    {
        SetLast(L"Unknown appearance mode.");
        return TaskbarFxErrBadMode;
    }

    const int kind = ProbeKind();
    if (kind == TaskbarFxKindNotFound)
    {
        return TaskbarFxErrNoTray;
    }
    if (kind == TaskbarFxKindModernXaml)
    {
        return ApplyWin11Xaml(mode, argb);
    }

    SetWindowCompositionAttributeFn fn = LoadSwca();
    if (fn == nullptr)
    {
        SetLast(L"SetWindowCompositionAttribute is not exported. This OS does not support the classic path.");
        return TaskbarFxErrMissingExport;
    }

    AccentPolicy policy{};
    if (!FillPolicy(mode, argb, &policy))
    {
        SetLast(L"Unknown appearance mode.");
        return TaskbarFxErrBadMode;
    }

    std::vector<HWND> trays = CollectTrays();
    int applied = 0;
    int failed = 0;
    for (HWND hwnd : trays)
    {
        if (HasXamlBridge(hwnd))
        {
            continue;
        }
        if (ApplyToHwnd(fn, hwnd, policy) == TaskbarFxOk)
        {
            applied += 1;
        }
        else
        {
            failed += 1;
        }
    }

    if (applied == 0)
    {
        SetLast(L"SetWindowCompositionAttribute failed on every taskbar window.");
        return TaskbarFxErrCallFailed;
    }

    if (failed > 0)
    {
        SetLast(L"Some secondary taskbars could not be updated.");
        return TaskbarFxOk;
    }

    if (mode == TaskbarFxModeNormal)
    {
        SetLast(L"Restored system default (did not end explorer).");
    }
    else
    {
        SetLast(L"Applied classic taskbar appearance.");
    }
    return TaskbarFxOk;
}

int __stdcall TaskbarFxReset(void)
{
    // Recovery = composition disabled on the tray HWND.
    // Forbidden as a default: ending explorer.exe.
    return TaskbarFxApply(TaskbarFxModeNormal, 0);
}

int __stdcall TaskbarFxLastMessage(wchar_t* buffer, int cch)
{
    if (buffer == nullptr || cch <= 0)
    {
        return 0;
    }
    wcsncpy_s(buffer, static_cast<size_t>(cch), g_lastMessage, _TRUNCATE);
    return static_cast<int>(wcsnlen_s(buffer, static_cast<size_t>(cch)));
}
