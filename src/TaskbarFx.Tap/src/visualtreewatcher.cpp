#include "visualtreewatcher.hpp"

#include <Windows.h>
#include <oleauto.h>
#include <thread>
#include <windows.ui.xaml.hosting.desktopwindowxamlsource.h>

#include "undefgetcurrenttime.h"
#include <winrt/Windows.UI.Xaml.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Hosting.h>
#include <winrt/Windows.UI.Xaml.Media.h>
#include <winrt/Windows.UI.Xaml.Shapes.h>
#include "redefgetcurrenttime.h"

#include <fstream>
#include <string>

namespace
{
    void TapLog(const std::wstring& line)
    {
        try
        {
            wchar_t tmp[MAX_PATH]{};
            GetTempPathW(MAX_PATH, tmp);
            std::wstring path(tmp);
            path += L"suite-tap-tree.log";
            std::wofstream out(path, std::ios::app);
            if (out)
            {
                out << line << L"\n";
            }
        }
        catch (...)
        {
        }
    }

    struct UniqueBstr
    {
        BSTR p = nullptr;
        explicit UniqueBstr(BSTR value) noexcept : p(value) {}
        ~UniqueBstr()
        {
            if (p)
            {
                SysFreeString(p);
            }
        }
        UniqueBstr(const UniqueBstr&) = delete;
        UniqueBstr& operator=(const UniqueBstr&) = delete;
        BSTR get() const noexcept { return p; }
        UINT len() const noexcept { return p ? SysStringLen(p) : 0; }
    };
}

VisualTreeWatcher::VisualTreeWatcher(winrt::com_ptr<IUnknown> site, HANDLE readyEvent) :
    m_XamlDiagnostics(site.as<IXamlDiagnostics>()),
    m_ReadyEvent(readyEvent)
{
    // Calling AdviseVisualTreeChange from a separate thread solves some hangs.
    // (TranslucentTB ExplorerTAP/visualtreewatcher.cpp)
    std::thread([self_strong = get_strong()]
    {
        // Advise walks the existing tree and delivers Add callbacks on this stack.
        // Ready must not fire before that, or Native Apply sees no BackgroundFill.
        winrt::check_hresult(self_strong->m_XamlDiagnostics.as<IVisualTreeService3>()->AdviseVisualTreeChange(self_strong.get()));
        AppearanceController::Instance().MarkReady();
        if (self_strong->m_ReadyEvent)
        {
            SetEvent(self_strong->m_ReadyEvent);
        }
    }).detach();
}

HRESULT VisualTreeWatcher::OnVisualTreeChange(ParentChildRelation relation, VisualElement element, VisualMutationType mutationType) try
{
    UniqueBstr filename(element.SrcInfo.FileName);
    UniqueBstr hash(element.SrcInfo.Hash);
    UniqueBstr name(element.Name);
    UniqueBstr type(element.Type);
    (void)filename;
    (void)hash;

    switch (mutationType)
    {
    case Add:
    {
        const std::wstring_view typeView{ type.get(), type.len() };
        const std::wstring_view nameViewEarly{ name.get(), name.len() };
        if (typeView.find(L"Taskbar") != std::wstring_view::npos
            || nameViewEarly.find(L"Background") != std::wstring_view::npos
            || typeView.find(L"Rectangle") != std::wstring_view::npos
            || typeView.find(L"Border") != std::wstring_view::npos)
        {
            TapLog(L"ADD type=" + std::wstring(typeView) + L" name=" + std::wstring(nameViewEarly));
        }
        if (typeView == winrt::name_of<wuxh::DesktopWindowXamlSource>())
        {
            m_NonMatchingXamlSources.insert(element.Handle);
        }
        else if (typeView == L"Taskbar.TaskbarFrame")
        {
            const auto rootGrid = FromHandle<wux::UIElement>(relation.Parent);
            bool matched = false;
            for (auto it = m_NonMatchingXamlSources.begin(); it != m_NonMatchingXamlSources.end(); ++it)
            {
                const auto xamlSource = FromHandle<wuxh::DesktopWindowXamlSource>(*it);
                wux::UIElement content = nullptr;
                try
                {
                    content = xamlSource.Content();
                }
                catch (const winrt::hresult_wrong_thread&)
                {
                    continue;
                }

                if (content == rootGrid)
                {
                    const auto nativeSource = xamlSource.as<IDesktopWindowXamlSourceNative>();
                    HWND hwnd = nullptr;
                    winrt::check_hresult(nativeSource->get_WindowHandle(&hwnd));
                    AppearanceController::Instance().RegisterTaskbar(element.Handle, hwnd);
                    try
                    {
                        const auto frame = FromHandle<wux::FrameworkElement>(element.Handle);
                        DiscoverFills(element.Handle, frame);
                    }
                    catch (...)
                    {
                    }
                    AppearanceController::Instance().ApplyPendingFromShm();
                    m_NonMatchingXamlSources.erase(it);
                    matched = true;
                    break;
                }
            }

            // XamlSource may not be in the set yet (advise order). Still register + scan fills.
            if (!matched)
            {
                HWND tray = FindWindowW(L"Shell_TrayWnd", nullptr);
                AppearanceController::Instance().RegisterTaskbar(element.Handle, tray);
                try
                {
                    const auto frame = FromHandle<wux::FrameworkElement>(element.Handle);
                    DiscoverFills(element.Handle, frame);
                }
                catch (...)
                {
                }
                AppearanceController::Instance().ApplyPendingFromShm();
            }
        }
        else
        {
            const std::wstring_view nameView{ name.get(), name.len() };
            const bool backgroundFill = nameView == L"BackgroundFill";
            const bool backgroundStroke = nameView == L"BackgroundStroke";
            if (backgroundFill || backgroundStroke)
            {
                wux::FrameworkElement self = nullptr;
                wux::FrameworkElement parent = nullptr;
                try
                {
                    self = FromHandle<wux::FrameworkElement>(element.Handle);
                    parent = FromHandle<wux::FrameworkElement>(relation.Parent);
                }
                catch (...)
                {
                    break;
                }

                if (const auto frame = FindParent(L"TaskbarFrame", parent))
                {
                    InstanceHandle handle = 0;
                    winrt::check_hresult(m_XamlDiagnostics->GetHandleFromIInspectable(
                        static_cast<::IInspectable*>(winrt::get_abi(frame)), &handle));
                    TryRegisterNamedBackground(handle, self);
                    AppearanceController::Instance().ApplyPendingFromShm();
                }
            }
        }
        break;
    }
    case Remove:
        AppearanceController::Instance().UnregisterTaskbar(element.Handle);
        m_NonMatchingXamlSources.erase(element.Handle);
        break;
    }

    return S_OK;
}
catch (...)
{
    return winrt::to_hresult();
}

HRESULT VisualTreeWatcher::OnElementStateChanged(InstanceHandle, VisualElementState, LPCWSTR) noexcept
{
    return S_OK;
}

wux::FrameworkElement VisualTreeWatcher::FindParent(std::wstring_view name, wux::FrameworkElement element)
{
    const auto parent = wux::Media::VisualTreeHelper::GetParent(element).try_as<wux::FrameworkElement>();
    if (!parent)
    {
        return nullptr;
    }
    try
    {
        if (parent.Name() == name)
        {
            return parent;
        }
        // Win11 builds may leave Name empty; type is still Taskbar.TaskbarFrame.
        const auto typeName = winrt::get_class_name(parent);
        if (name == L"TaskbarFrame"
            && (typeName == L"Taskbar.TaskbarFrame" || typeName == L"TaskbarFrame"))
        {
            return parent;
        }
    }
    catch (...)
    {
    }
    return FindParent(name, parent);
}

void VisualTreeWatcher::DiscoverFills(InstanceHandle frameHandle, wux::FrameworkElement root) try
{
    if (!root)
    {
        return;
    }

    TryRegisterNamedBackground(frameHandle, root);

    const int count = wux::Media::VisualTreeHelper::GetChildrenCount(root);
    for (int i = 0; i < count; ++i)
    {
        const auto child = wux::Media::VisualTreeHelper::GetChild(root, i);
        if (const auto fe = child.try_as<wux::FrameworkElement>())
        {
            DiscoverFills(frameHandle, fe);
        }
    }
}
catch (...)
{
}

void VisualTreeWatcher::TryRegisterNamedBackground(InstanceHandle frameHandle, wux::FrameworkElement element)
{
    if (!element)
    {
        return;
    }

    winrt::hstring name;
    try
    {
        name = element.Name();
    }
    catch (...)
    {
        return;
    }

    const bool fill = name == L"BackgroundFill";
    const bool stroke = name == L"BackgroundStroke";
    if (!fill && !stroke)
    {
        return;
    }

    // Register by name. Shape/Border preferred; other FE types still recorded so PendingFills clears.
    // WriteBrush applies only when Shape/Border (or extended) is supported.
    if (fill)
    {
        AppearanceController::Instance().RegisterTaskbarBackground(frameHandle, element);
    }
    else
    {
        AppearanceController::Instance().RegisterTaskbarBorder(frameHandle, element);
    }
}
