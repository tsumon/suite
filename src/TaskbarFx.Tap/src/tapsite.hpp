#pragma once

// Copyright (C) TranslucentTB contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Derived from TranslucentTB ExplorerTAP/tapsite.hpp
// https://github.com/TranslucentTB/TranslucentTB
// commit d4636e439865df0a1a1419db408e055740ce5c74
//
// Modifications Copyright (C) 2026 Joe / Suite contributors
// Unique CLSID / event names. No Detours payload.

#include <Windows.h>
#include <ocidl.h>
#include <xamlOM.h>
#include "winrt_alias.hpp"
#include "visualtreewatcher.hpp"

class TAPSite : public winrt::implements<TAPSite, IObjectWithSite, winrt::non_agile>
{
public:
    static HANDLE GetReadyEvent();
    static DWORD WINAPI Install(void* parameter);
    static bool SiteReady();
    static void StartPump();

private:
    HRESULT STDMETHODCALLTYPE SetSite(IUnknown* pUnkSite) override;
    HRESULT STDMETHODCALLTYPE GetSite(REFIID riid, void** ppvSite) noexcept override;

    static winrt::weak_ref<VisualTreeWatcher> s_VisualTreeWatcher;
    static volatile LONG s_SiteReady;

    winrt::com_ptr<IUnknown> site;
};
