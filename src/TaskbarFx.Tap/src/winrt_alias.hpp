#pragma once

// Copyright (C) TranslucentTB contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Derived from TranslucentTB Common/winrt.hpp
// https://github.com/TranslucentTB/TranslucentTB
// commit d4636e439865df0a1a1419db408e055740ce5c74
//
// Modifications Copyright (C) 2026 Joe / Suite contributors
// Namespace aliases only. No TranslucentTB XAML UI types.

#include <guiddef.h>
#include <Unknwn.h>
#include <winrt/base.h>

namespace winrt {
    namespace Windows {
        namespace Foundation {}
        namespace System {}
        namespace UI {
            namespace Xaml {
                namespace Controls {}
                namespace Hosting {}
                namespace Media {}
                namespace Shapes {}
            }
        }
    }
}

namespace wf = winrt::Windows::Foundation;
namespace wux = winrt::Windows::UI::Xaml;
namespace wuxh = wux::Hosting;
