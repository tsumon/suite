#include "tapsite.hpp"

#include <Windows.h>
#include <string>
#include <thread>

#include "../../TaskbarFx.Shared/TaskbarFxTapProtocol.h"

using PFN_INITIALIZE_XAML_DIAGNOSTICS_EX = decltype(&InitializeXamlDiagnosticsEx);

winrt::weak_ref<VisualTreeWatcher> TAPSite::s_VisualTreeWatcher;
volatile LONG TAPSite::s_SiteReady = 0;

namespace
{
    std::wstring ModulePath()
    {
        HMODULE module = nullptr;
        if (!GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            reinterpret_cast<LPCWSTR>(&TAPSite::Install),
            &module)
            || module == nullptr)
        {
            return {};
        }

        wchar_t path[MAX_PATH];
        const DWORD n = GetModuleFileNameW(module, path, MAX_PATH);
        if (n == 0 || n >= MAX_PATH)
        {
            return {};
        }
        return path;
    }

    HANDLE OpenOrCreateReadyEvent()
    {
        HANDLE event = OpenEventW(EVENT_MODIFY_STATE | SYNCHRONIZE, FALSE, kTaskbarFxTapReadyEventName);
        if (event == nullptr)
        {
            event = CreateEventW(nullptr, TRUE, FALSE, kTaskbarFxTapReadyEventName);
        }
        return event;
    }
}

HANDLE TAPSite::GetReadyEvent()
{
    return OpenOrCreateReadyEvent();
}

bool TAPSite::SiteReady()
{
    return s_SiteReady != 0;
}

DWORD TAPSite::Install(void*)
{
    // Ready is signaled only after AdviseVisualTreeChange (VisualTreeWatcher).
    // Do not SetEvent on failure — Native WaitReady must time out cleanly.
    const std::wstring location = ModulePath();
    if (location.empty())
    {
        return HRESULT_FROM_WIN32(ERROR_MOD_NOT_FOUND);
    }

    HMODULE wux = LoadLibraryExW(L"Windows.UI.Xaml.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (wux == nullptr)
    {
        return HRESULT_FROM_WIN32(GetLastError());
    }

    const auto ixde = reinterpret_cast<PFN_INITIALIZE_XAML_DIAGNOSTICS_EX>(
        GetProcAddress(wux, "InitializeXamlDiagnosticsEx"));
    if (ixde == nullptr)
    {
        return HRESULT_FROM_WIN32(GetLastError());
    }

    const DWORD pid = GetCurrentProcessId();
    HRESULT hr = E_FAIL;
    // XAML Diagnostics can only be initialized once per thread; each attempt needs a fresh thread.
    // (Same pattern as TranslucentTB ExplorerTAP/tapsite.cpp)
    for (int attempt = 1; attempt <= 60; ++attempt)
    {
        wchar_t conn[64];
        swprintf_s(conn, L"VisualDiagConnection%d", attempt);
        std::thread([&hr, ixde, pid, &location, conn]
        {
            hr = ixde(conn, pid, nullptr, location.c_str(), CLSID_TaskbarFxTapSite, nullptr);
        }).join();
        if (SUCCEEDED(hr))
        {
            return S_OK;
        }
        Sleep(500);
    }

    return hr;
}

HRESULT TAPSite::SetSite(IUnknown* pUnkSite) try
{
    if (s_VisualTreeWatcher.get())
    {
        throw winrt::hresult_illegal_method_call();
    }

    site.copy_from(pUnkSite);
    if (site)
    {
        StartPump();
        const HANDLE ready = GetReadyEvent();
        // VisualTreeWatcher signals ready after AdviseVisualTreeChange returns.
        s_VisualTreeWatcher = winrt::make_self<VisualTreeWatcher>(site, ready);
        InterlockedExchange(&s_SiteReady, 1);
    }
    return S_OK;
}
catch (...)
{
    return winrt::to_hresult();
}

HRESULT TAPSite::GetSite(REFIID riid, void** ppvSite) noexcept
{
    return site.as(riid, ppvSite);
}
