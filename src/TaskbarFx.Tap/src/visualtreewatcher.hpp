#pragma once

// Copyright (C) TranslucentTB contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Derived from TranslucentTB ExplorerTAP/visualtreewatcher.hpp
// https://github.com/TranslucentTB/TranslucentTB
// commit d4636e439865df0a1a1419db408e055740ce5c74
//
// Modifications Copyright (C) 2026 Joe / Suite contributors

#include <Windows.h>
#include <string_view>
#include <unordered_set>
#include <xamlOM.h>

#include "winrt_alias.hpp"
#include "undefgetcurrenttime.h"
#include <winrt/Windows.UI.Xaml.h>
#include "redefgetcurrenttime.h"

#include "appearance.hpp"

struct VisualTreeWatcher : winrt::implements<VisualTreeWatcher, IVisualTreeServiceCallback2, winrt::non_agile>
{
    VisualTreeWatcher(winrt::com_ptr<IUnknown> site, HANDLE readyEvent);

    VisualTreeWatcher(const VisualTreeWatcher&) = delete;
    VisualTreeWatcher& operator=(const VisualTreeWatcher&) = delete;

private:
    HRESULT STDMETHODCALLTYPE OnVisualTreeChange(ParentChildRelation relation, VisualElement element, VisualMutationType mutationType) override;
    HRESULT STDMETHODCALLTYPE OnElementStateChanged(InstanceHandle element, VisualElementState elementState, LPCWSTR context) noexcept override;

    wux::FrameworkElement FindParent(std::wstring_view name, wux::FrameworkElement element);
    void DiscoverFills(InstanceHandle frameHandle, wux::FrameworkElement root);
    void TryRegisterNamedBackground(InstanceHandle frameHandle, wux::FrameworkElement element);

    template<typename T>
    T FromHandle(InstanceHandle handle)
    {
        wf::IInspectable obj;
        winrt::check_hresult(m_XamlDiagnostics->GetIInspectableFromHandle(
            handle, reinterpret_cast<::IInspectable**>(winrt::put_abi(obj))));
        return obj.as<T>();
    }

    winrt::com_ptr<IXamlDiagnostics> m_XamlDiagnostics;
    std::unordered_set<InstanceHandle> m_NonMatchingXamlSources;
    HANDLE m_ReadyEvent = nullptr;
};
