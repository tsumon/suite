#include "GaussianBlurEffect.h"

HRESULT GaussianBlurEffect::GetEffectId(GUID* id) noexcept
{
	if (id == nullptr) [[unlikely]]
	{
		return E_INVALIDARG;
	}

	*id = CLSID_D2D1GaussianBlur;
	return S_OK;
}

HRESULT GaussianBlurEffect::GetNamedPropertyMapping(LPCWSTR name, UINT* index, awge::GRAPHICS_EFFECT_PROPERTY_MAPPING* mapping) noexcept
{
	if (index == nullptr || mapping == nullptr) [[unlikely]]
	{
		return E_INVALIDARG;
	}

	const std::wstring_view nameView(name);
	if (nameView == L"BlurAmount")
	{
		*index = D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION;
		*mapping = awge::GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT;

		return S_OK;
	}
	else if (nameView == L"Optimization")
	{
		*index = D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION;
		*mapping = awge::GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT;

		return S_OK;
	}
	else if (nameView == L"BorderMode")
	{
		*index = D2D1_GAUSSIANBLUR_PROP_BORDER_MODE;
		*mapping = awge::GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT;

		return S_OK;
	}

	return E_INVALIDARG;
}

HRESULT GaussianBlurEffect::GetPropertyCount(UINT* count) noexcept
{
	if (count == nullptr) [[unlikely]]
	{
		return E_INVALIDARG;
	}

	*count = 3;
	return S_OK;
}

HRESULT GaussianBlurEffect::GetProperty(UINT index, ABI::Windows::Foundation::IPropertyValue** value) noexcept try
{
	if (value == nullptr) [[unlikely]]
	{
		return E_INVALIDARG;
	}

	switch (index)
	{
		case D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION:
			*value = wf::PropertyValue::CreateSingle(BlurAmount).as<ABI::Windows::Foundation::IPropertyValue>().detach();
			break;

		case D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION:
			*value = wf::PropertyValue::CreateUInt32((UINT32)Optimization).as<ABI::Windows::Foundation::IPropertyValue>().detach();
			break;

		case D2D1_GAUSSIANBLUR_PROP_BORDER_MODE:
			*value = wf::PropertyValue::CreateUInt32((UINT32)BorderMode).as<ABI::Windows::Foundation::IPropertyValue>().detach();
			break;

		default:
			return E_BOUNDS;
	}

	return S_OK;
}
catch (...)
{
	return winrt::to_hresult();
}

HRESULT GaussianBlurEffect::GetSource(UINT index, awge::IGraphicsEffectSource** source) noexcept
{
	if (source == nullptr) [[unlikely]]
	{
		return E_INVALIDARG;
	}

	if (index == 0)
	{
		winrt::copy_to_abi(Source, *reinterpret_cast<void**>(source));
		return S_OK;
	}
	else
	{
		return E_BOUNDS;
	}
}

HRESULT GaussianBlurEffect::GetSourceCount(UINT* count) noexcept
{
	if (count == nullptr) [[unlikely]]
	{
		return E_INVALIDARG;
	}

	*count = 1;
	return S_OK;
}

winrt::hstring GaussianBlurEffect::Name()
{
	return m_name;
}

void GaussianBlurEffect::Name(winrt::hstring name)
{
	m_name = name;
}
