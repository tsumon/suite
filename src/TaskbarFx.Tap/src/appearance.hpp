#pragma once

// Copyright (C) TranslucentTB contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Derived from TranslucentTB ExplorerTAP/taskbarappearanceservice.hpp
// https://github.com/TranslucentTB/TranslucentTB
// commit d4636e439865df0a1a1419db408e055740ce5c74
//
// Modifications Copyright (C) 2026 Joe / Suite contributors
// Suite-only: no MIDL proxy, no package-uninstall explorer kill, no Blur brush.

#include <Windows.h>
#include <unordered_map>
#include <xamlOM.h>

#include "winrt_alias.hpp"
#include "undefgetcurrenttime.h"
#include <winrt/Windows.UI.Xaml.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Media.h>
#include <winrt/Windows.UI.Xaml.Shapes.h>
#include "redefgetcurrenttime.h"

class AppearanceController
{
public:
    static AppearanceController& Instance();

    AppearanceController(const AppearanceController&) = delete;
    AppearanceController& operator=(const AppearanceController&) = delete;

    void RegisterTaskbar(InstanceHandle frameHandle, HWND window);
    void RegisterTaskbarBackground(InstanceHandle frameHandle, wux::FrameworkElement element);
    void RegisterTaskbarBorder(InstanceHandle frameHandle, wux::FrameworkElement element);
    void UnregisterTaskbar(InstanceHandle frameHandle);

    int ApplyPendingFromShm();
    void RestoreAll();
    void AttachShm();
    void MarkReady();

private:
    AppearanceController() = default;

    struct ControlInfo
    {
        wux::FrameworkElement control = nullptr;
        wux::Media::Brush originalFill = nullptr;
        bool stroke = false;
    };

    struct TaskbarInfo
    {
        ControlInfo background;
        ControlInfo border;
        HWND window = nullptr;
    };

    void ApplyOne(TaskbarInfo& info, int mode, unsigned int argb, bool* acrylicFellBack);
    static void RestoreFill(const ControlInfo& info);
    static wux::Media::Brush ReadBrush(const wux::FrameworkElement& element, bool stroke);
    static void WriteBrush(const wux::FrameworkElement& element, const wux::Media::Brush& brush, bool stroke);
    static winrt::Windows::UI::Color ColorFromArgb(unsigned int argb);
    static wux::Media::Brush MakeBrush(int mode, unsigned int argb, bool* acrylicFellBack);

    std::unordered_map<InstanceHandle, TaskbarInfo> m_taskbars;
};
