#include "tapsite.hpp"
#include <format>

#include "constants.hpp"
#include "util/string_macros.hpp"
#include "win32.hpp"

using PFN_INITIALIZE_XAML_DIAGNOSTICS_EX = decltype(&InitializeXamlDiagnosticsEx);

winrt::weak_ref<VisualTreeWatcher> TAPSite::s_VisualTreeWatcher;

wil::unique_event_nothrow TAPSite::GetReadyEvent()
{
	wil::unique_event_nothrow readyEvent;
	winrt::check_hresult(readyEvent.create(wil::EventOptions::ManualReset, TAP_READY_EVENT.c_str()));
	return readyEvent;
}

DWORD TAPSite::Install(void*)
{
	const auto event = GetReadyEvent();

	auto [location, hr] = win32::GetDllLocation(wil::GetModuleInstanceHandle());
	if (FAILED(hr)) [[unlikely]]
	{
		event.SetEvent();
		return hr;
	}

	const wil::unique_hmodule wux(LoadLibraryEx(L"Windows.UI.Xaml.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32));
	if (!wux) [[unlikely]]
	{
		event.SetEvent();
		return HRESULT_FROM_WIN32(GetLastError());
	}

	const auto ixde = reinterpret_cast<PFN_INITIALIZE_XAML_DIAGNOSTICS_EX>(GetProcAddress(wux.get(), UTIL_STRINGIFY_UTF8(InitializeXamlDiagnosticsEx)));
	if (!ixde) [[unlikely]]
	{
		event.SetEvent();
		return HRESULT_FROM_WIN32(GetLastError());
	}

	DWORD pid = GetCurrentProcessId();
	uint8_t attempts = 1;
	do
	{
		// We need this to exist because XAML Diagnostics can only be initialized once per thread
		// future calls simply return S_OK without doing anything.
		std::thread([&hr, ixde, pid, &location, conn = std::format(L"VisualDiagConnection{}", static_cast<int>(attempts))]
		{
			hr = ixde(conn.c_str(), pid, nullptr, location.c_str(), CLSID_TAPSite, nullptr);
		}).join();

		if (SUCCEEDED(hr))
		{
			return S_OK;
		}
		else
		{
			++attempts;
			Sleep(500);
		}
	} while (FAILED(hr) && attempts <= 60);
	// 60 * 500ms = 30s

	event.SetEvent();
	return hr;
}

HRESULT TAPSite::SetSite(IUnknown *pUnkSite) try
{
	// only ever 1 VTW at once
	if (s_VisualTreeWatcher.get())
	{
		throw winrt::hresult_illegal_method_call();
	}

	site.copy_from(pUnkSite);

	if (site)
	{
		s_VisualTreeWatcher = winrt::make_self<VisualTreeWatcher>(site, GetReadyEvent());
	}

	return S_OK;
}
catch (...)
{
	return winrt::to_hresult();
}

HRESULT TAPSite::GetSite(REFIID riid, void **ppvSite) noexcept
{
	return site.as(riid, ppvSite);
}
