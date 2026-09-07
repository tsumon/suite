#include "appearance.hpp"

#include <Windows.h>
#include <cstdint>

#include "undefgetcurrenttime.h"
#include <winrt/Windows.UI.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Shapes.h>
#include "redefgetcurrenttime.h"

#include "../../TaskbarFx.Shared/TaskbarFxTapProtocol.h"

namespace
{
    HANDLE g_shmHandle = nullptr;
    TaskbarFxTapShm* g_shmView = nullptr;
}

AppearanceController& AppearanceController::Instance()
{
    static AppearanceController instance;
    return instance;
}

void AppearanceController::AttachShm()
{
    if (g_shmView)
    {
        // ready is set only after AdviseVisualTreeChange (MarkReady).
        g_shmView->explorerPid = GetCurrentProcessId();
        return;
    }

    g_shmHandle = OpenFileMappingW(FILE_MAP_READ | FILE_MAP_WRITE, FALSE, kTaskbarFxTapShmName);
    if (g_shmHandle == nullptr)
    {
        g_shmHandle = CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            nullptr,
            PAGE_READWRITE,
            0,
            sizeof(TaskbarFxTapShm),
            kTaskbarFxTapShmName);
    }
    if (g_shmHandle == nullptr)
    {
        return;
    }

    g_shmView = static_cast<TaskbarFxTapShm*>(MapViewOfFile(
        g_shmHandle, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(TaskbarFxTapShm)));
    if (g_shmView == nullptr)
    {
        return;
    }

    if (g_shmView->magic != kTaskbarFxTapMagic)
    {
        ZeroMemory(g_shmView, sizeof(TaskbarFxTapShm));
        g_shmView->magic = kTaskbarFxTapMagic;
        g_shmView->version = kTaskbarFxTapVersion;
    }
    g_shmView->explorerPid = GetCurrentProcessId();
}

void AppearanceController::MarkReady()
{
    if (g_shmView)
    {
        g_shmView->ready = 1;
        g_shmView->explorerPid = GetCurrentProcessId();
    }
}

void AppearanceController::RegisterTaskbar(InstanceHandle frameHandle, HWND window)
{
    m_taskbars.insert_or_assign(frameHandle, TaskbarInfo{ {}, {}, window });
}

void AppearanceController::RegisterTaskbarBackground(InstanceHandle frameHandle, wux::FrameworkElement element)
{
    if (!element)
    {
        return;
    }

    auto it = m_taskbars.find(frameHandle);
    if (it == m_taskbars.end())
    {
        // BackgroundFill may arrive before TaskbarFrame matching finishes.
        m_taskbars.insert_or_assign(frameHandle, TaskbarInfo{ {}, {}, nullptr });
        it = m_taskbars.find(frameHandle);
    }

    it->second.background.control = element;
    it->second.background.stroke = false;
    it->second.background.originalFill = ReadBrush(element, false);
}

void AppearanceController::RegisterTaskbarBorder(InstanceHandle frameHandle, wux::FrameworkElement element)
{
    if (!element)
    {
        return;
    }

    auto it = m_taskbars.find(frameHandle);
    if (it == m_taskbars.end())
    {
        m_taskbars.insert_or_assign(frameHandle, TaskbarInfo{ {}, {}, nullptr });
        it = m_taskbars.find(frameHandle);
    }

    it->second.border.control = element;
    it->second.border.stroke = true;
    it->second.border.originalFill = ReadBrush(element, true);
}

void AppearanceController::UnregisterTaskbar(InstanceHandle frameHandle)
{
    m_taskbars.erase(frameHandle);
}

void AppearanceController::RestoreAll()
{
    for (auto& [handle, info] : m_taskbars)
    {
        RestoreFill(info.background);
        RestoreFill(info.border);
    }
}

int AppearanceController::ApplyPendingFromShm()
{
    HANDLE mapping = OpenFileMappingW(FILE_MAP_READ | FILE_MAP_WRITE, FALSE, kTaskbarFxTapShmName);
    if (mapping == nullptr)
    {
        return TaskbarFxTapResultFailed;
    }

    auto* shm = static_cast<TaskbarFxTapShm*>(MapViewOfFile(mapping, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(TaskbarFxTapShm)));
    if (shm == nullptr)
    {
        CloseHandle(mapping);
        return TaskbarFxTapResultFailed;
    }

    const int mode = shm->mode;
    const unsigned int argb = shm->argb;
    shm->fillCount = 0;
    for (const auto& [handle, info] : m_taskbars)
    {
        if (info.background.control)
        {
            shm->fillCount += 1;
        }
    }

    int result = TaskbarFxTapResultOk;
    try
    {
        if (mode == TaskbarFxTapModeNormal)
        {
            RestoreAll();
        }
        else
        {
            bool anyFill = false;
            bool acrylicFellBack = false;
            for (auto& [handle, info] : m_taskbars)
            {
                if (!info.background.control)
                {
                    continue;
                }
                anyFill = true;
                ApplyOne(info, mode, argb, &acrylicFellBack);
            }
            if (!anyFill)
            {
                result = TaskbarFxTapResultPendingFills;
            }
            else if (acrylicFellBack)
            {
                result = TaskbarFxTapResultAcrylicFallback;
            }
        }
    }
    catch (...)
    {
        result = TaskbarFxTapResultFailed;
    }

    shm->result = result;
    shm->ack = shm->seq;
    UnmapViewOfFile(shm);
    CloseHandle(mapping);
    return result;
}

void AppearanceController::ApplyOne(TaskbarInfo& info, int mode, unsigned int argb, bool* acrylicFellBack)
{
    if (!info.background.control)
    {
        return;
    }

    wux::Media::Brush brush = MakeBrush(mode, argb, acrylicFellBack);
    if (brush)
    {
        WriteBrush(info.background.control, brush, info.background.stroke);
    }

    if (info.border.control)
    {
        if (mode == TaskbarFxTapModeClear || mode == TaskbarFxTapModeAcrylic)
        {
            wux::Media::SolidColorBrush hidden;
            hidden.Opacity(0);
            WriteBrush(info.border.control, hidden, info.border.stroke);
        }
        else
        {
            RestoreFill(info.border);
        }
    }
}

void AppearanceController::RestoreFill(const ControlInfo& info)
{
    if (info.control && info.originalFill)
    {
        WriteBrush(info.control, info.originalFill, info.stroke);
    }
}

wux::Media::Brush AppearanceController::ReadBrush(const wux::FrameworkElement& element, bool stroke)
{
    if (const auto shape = element.try_as<wux::Shapes::Shape>())
    {
        return shape.Fill();
    }
    if (const auto border = element.try_as<wux::Controls::Border>())
    {
        return stroke ? border.BorderBrush() : border.Background();
    }
    if (!stroke)
    {
        if (const auto panel = element.try_as<wux::Controls::Panel>())
        {
            return panel.Background();
        }
    }
    return nullptr;
}

void AppearanceController::WriteBrush(const wux::FrameworkElement& element, const wux::Media::Brush& brush, bool stroke)
{
    if (const auto shape = element.try_as<wux::Shapes::Shape>())
    {
        shape.Fill(brush);
        return;
    }
    if (const auto border = element.try_as<wux::Controls::Border>())
    {
        if (stroke)
        {
            border.BorderBrush(brush);
        }
        else
        {
            border.Background(brush);
        }
        return;
    }
    if (!stroke)
    {
        if (const auto panel = element.try_as<wux::Controls::Panel>())
        {
            panel.Background(brush);
        }
    }
}

winrt::Windows::UI::Color AppearanceController::ColorFromArgb(unsigned int argb)
{
    winrt::Windows::UI::Color color{};
    color.A = static_cast<uint8_t>((argb >> 24) & 0xFFu);
    color.R = static_cast<uint8_t>((argb >> 16) & 0xFFu);
    color.G = static_cast<uint8_t>((argb >> 8) & 0xFFu);
    color.B = static_cast<uint8_t>(argb & 0xFFu);
    return color;
}

wux::Media::Brush AppearanceController::MakeBrush(int mode, unsigned int argb, bool* acrylicFellBack)
{
    winrt::Windows::UI::Color tint = ColorFromArgb(argb);

    if (mode == TaskbarFxTapModeOpaque)
    {
        tint.A = 0xFF;
        wux::Media::SolidColorBrush solid;
        solid.Color(tint);
        return solid;
    }

    if (mode == TaskbarFxTapModeClear)
    {
        tint.A = 0;
        wux::Media::SolidColorBrush solid;
        solid.Color(tint);
        return solid;
    }

    if (mode == TaskbarFxTapModeAcrylic)
    {
        if (tint.A == 0)
        {
            tint.A = 1;
        }
        try
        {
            wux::Media::AcrylicBrush acrylic;
            acrylic.BackgroundSource(wux::Media::AcrylicBackgroundSource::Backdrop);
            acrylic.TintColor(tint);
            return acrylic;
        }
        catch (...)
        {
            if (acrylicFellBack)
            {
                *acrylicFellBack = true;
            }
            wux::Media::SolidColorBrush solid;
            solid.Color(tint);
            return solid;
        }
    }

    return nullptr;
}
