#include "arch.h"
#include <libloaderapi.h>
#include <windef.h>
#include <processthreadsapi.h>
#include <detours/detours.h>

#include "constants.hpp"
#include "tapsite.hpp"
#include "taskbarappearanceservice.hpp"

BOOL WINAPI DllMain(HINSTANCE, DWORD fdwReason, LPVOID) noexcept
{
	switch (fdwReason)
	{
	case DLL_PROCESS_ATTACH:
	{
		// Are we in Explorer?
		void* payload = DetourFindPayloadEx(EXPLORER_PAYLOAD, nullptr);
		if (payload)
		{
			// Install the thing
			wil::unique_handle handle(CreateThread(nullptr, 0, TAPSite::Install, nullptr, 0, nullptr));
			if (!handle)
			{
				return false;
			}
		}
		break;
	}

	case DLL_PROCESS_DETACH:
		TaskbarAppearanceService::UninstallProxyStub();
		break;
	}

	return true;
}
