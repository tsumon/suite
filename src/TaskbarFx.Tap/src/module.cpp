#include <combaseapi.h>
#include "winrt_alias.hpp"
#include "tapsite.hpp"
#include "simplefactory.hpp"
#include "../../TaskbarFx.Shared/TaskbarFxTapProtocol.h"

_Use_decl_annotations_ STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, LPVOID* ppv) try
{
    if (rclsid == CLSID_TaskbarFxTapSite)
    {
        *ppv = nullptr;
        return winrt::make<SimpleFactory<TAPSite>>().as(riid, ppv);
    }
    return CLASS_E_CLASSNOTAVAILABLE;
}
catch (...)
{
    return winrt::to_hresult();
}

_Use_decl_annotations_ STDAPI DllCanUnloadNow()
{
    return S_FALSE;
}
