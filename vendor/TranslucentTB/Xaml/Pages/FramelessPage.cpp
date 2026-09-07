#include "pch.h"
#include <ranges>

#include "FramelessPage.h"
#if __has_include("Pages/FramelessPage.g.cpp")
#include "Pages/FramelessPage.g.cpp"
#endif

namespace winrt::TranslucentTB::Xaml::Pages::implementation
{
	void FramelessPage::InitializeComponent()
	{
		m_SystemMenuChangedToken = m_SystemMenuContent.VectorChanged({ this, &FramelessPage::SystemMenuChanged });

		FramelessPageT::InitializeComponent();
	}

	bool FramelessPage::CanMove() noexcept
	{
		return CanMoveCore();
	}

	bool FramelessPage::CanMoveCore() noexcept
	{
		return true;
	}

	void FramelessPage::ShowSystemMenu(wf::Point position)
	{
		if (CanMove())
		{
			if (FlowDirection() == wux::FlowDirection::RightToLeft)
			{
				position.X = static_cast<float>(ActualWidth() - position.X);
			}

			SystemMenu().ShowAt(*this, position);
		}
	}

	void FramelessPage::HideSystemMenu()
	{
		SystemMenu().Hide();
	}

	wf::Rect FramelessPage::DragRegion()
	{
		if (!ExpandIntoTitlebar())
		{
			const auto titlebar = Titlebar();
			const double titlebarHeight = titlebar.ActualHeight();
			const float controlsPosition = CustomTitlebarControls().ActualOffset().x;

			return {
				FlowDirection() == wux::FlowDirection::LeftToRight ? 0.0f : static_cast<float>(titlebar.ActualWidth() - controlsPosition),
				0.0f,
				controlsPosition,
				static_cast<float>(titlebarHeight)
			};
		}
		else
		{
			return ExpandedDragRegion();
		}
	}

	wf::Rect FramelessPage::ExpandedDragRegion()
	{
		throw hresult_not_implemented(L"A page that uses ExpandIntoTitlebar should override ExpandedDragRegion.");
	}

	wf::Rect FramelessPage::TitlebarButtonsRegion()
	{
		const bool closable = IsClosable();
		if (!ExpandIntoTitlebar() || (closable == false && m_TitlebarContent.Size() == 0))
		{
			return { 0.0f, 0.0f, 0.0f, 0.0f };
		}
		else
		{
			double height = 0.0f;
			double width = 0.0f;
			if (closable)
			{
				const auto closeButton = CloseButton();

				width += closeButton.ActualWidth();
				height = std::max(height, closeButton.ActualHeight());
			}

			for (const auto button : m_TitlebarContent)
			{
				width += button.ActualWidth();
				height = std::max(height, button.ActualHeight());
			}

			return {
				FlowDirection() == wux::FlowDirection::LeftToRight ? static_cast<float>(ActualWidth() - width) : 0.0f,
				0.0f,
				static_cast<float>(width),
				static_cast<float>(height)
			};
		}
	}

	bool FramelessPage::RequestClose()
	{
		return Close();
	}

	bool FramelessPage::Close()
	{
		if (IsClosable())
		{
			m_ClosedHandler();
			return true;
		}
		else
		{
			return false;
		}
	}

	void FramelessPage::CloseClicked(const IInspectable &, const wux::RoutedEventArgs &)
	{
		Close();
	}

	void FramelessPage::SystemMenuOpening(const IInspectable &, const IInspectable &)
	{
		if (m_NeedsSystemMenuRefresh)
		{
			const auto menuItems = SystemMenu().Items();

			// remove everything but the Close menu item
			const auto closeItem = CloseMenuItem();
			menuItems.Clear();
			menuItems.Append(closeItem);

			bool needsMergeStyle = false;
			if (m_SystemMenuContent.Size() > 0)
			{
				menuItems.InsertAt(0, wuxc::MenuFlyoutSeparator());
				for (const auto newItem : m_SystemMenuContent | std::views::reverse)
				{
					if (newItem.try_as<wuxc::ToggleMenuFlyoutItem>())
					{
						needsMergeStyle = true;
					}

					menuItems.InsertAt(0, newItem);
				}
			}

			closeItem.Style(needsMergeStyle ? LookupStyle(winrt::box_value(L"MergeIconsMenuFlyoutItem")) : nullptr);

			m_NeedsSystemMenuRefresh = false;
		}
	}

	FramelessPage::~FramelessPage()
	{
		m_SystemMenuContent.VectorChanged(m_SystemMenuChangedToken);
	}

	void FramelessPage::SystemMenuChanged(const wfc::IObservableVector<wuxc::MenuFlyoutItemBase> &, const wfc::IVectorChangedEventArgs &)
	{
		m_NeedsSystemMenuRefresh = true;
	}

	wux::Style FramelessPage::LookupStyle(const IInspectable &key)
	{
		return wux::Application::Current().Resources().TryLookup(key).try_as<wux::Style>();
	}
}
