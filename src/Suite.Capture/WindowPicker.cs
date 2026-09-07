using System.Text;
using System.Windows.Automation;
using Suite.Capture.Native;

namespace Suite.Capture;

/// <summary>
/// WeChat-style hover pick: EnumWindows Z-order + WindowFromPoint + UI Automation.
/// Skips overlay HWNDs without ShowWindow(SW_HIDE) — hiding broke mouse-move / looked like snap was gone.
/// Does not reverse-engineer Snipaste.
/// </summary>
internal static class WindowPicker
{
    private static readonly string[] DesktopClasses =
    [
        "Progman",
        "WorkerW",
        "#32769",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
    ];

    public static PixelRect Hit(
        int screenX,
        int screenY,
        IReadOnlyList<IntPtr> ignoreHwnds,
        IReadOnlyList<MonitorInfo> monitors)
    {
        // Primary: Z-order walk skipping overlays (no hide / no flicker / mouse stays on overlay).
        PixelRect byEnum = HitByZOrder(screenX, screenY, ignoreHwnds);
        if (!byEnum.IsEmpty)
        {
            return PreferUiaIfSmaller(byEnum, screenX, screenY);
        }

        // Fallback: temporarily mark overlays transparent (no Hide), then WindowFromPoint.
        PixelRect? found = null;
        RunThroughOverlays(ignoreHwnds, () =>
        {
            found = HitUnlocked(screenX, screenY, ignoreHwnds, monitors);
        });
        if (found is PixelRect pierced && !pierced.IsEmpty)
        {
            return pierced;
        }

        // Desktop / empty: highlight work area (CAPTURE-PIN §7.1). Full-screen highlight only when no window.
        return WorkAreaAt(screenX, screenY, monitors);
    }

    private static PixelRect PreferUiaIfSmaller(PixelRect windowRect, int screenX, int screenY)
    {
        PixelRect? uia = TryAutomation(screenX, screenY);
        if (uia is PixelRect inner
            && !inner.IsEmpty
            && inner.Width * inner.Height <= windowRect.Width * windowRect.Height
            && inner.Width >= CaptureUx.MinCommitPx
            && inner.Height >= CaptureUx.MinCommitPx)
        {
            return inner;
        }

        return windowRect;
    }

    private static PixelRect HitByZOrder(int screenX, int screenY, IReadOnlyList<IntPtr> ignoreHwnds)
    {
        IntPtr hit = IntPtr.Zero;
        CaptureNative.EnumWindows(
            (hwnd, _) =>
            {
                if (hwnd == IntPtr.Zero || Contains(ignoreHwnds, hwnd))
                {
                    return true;
                }

                if (!IsUsableWindow(hwnd) || !TryRect(hwnd, out PixelRect bounds) || bounds.IsEmpty)
                {
                    return true;
                }

                if (!bounds.Contains(screenX, screenY))
                {
                    return true;
                }

                hit = hwnd;
                return false;
            },
            IntPtr.Zero);

        if (hit == IntPtr.Zero)
        {
            return default;
        }

        IntPtr deep = DeepestChild(hit, screenX, screenY);
        if (deep != IntPtr.Zero && IsUsableWindow(deep) && TryRect(deep, out PixelRect child) && !child.IsEmpty)
        {
            if (child.Width >= CaptureUx.MinCommitPx && child.Height >= CaptureUx.MinCommitPx)
            {
                return child;
            }
        }

        if (TryRect(hit, out PixelRect top) && !top.IsEmpty
            && top.Width >= CaptureUx.MinCommitPx && top.Height >= CaptureUx.MinCommitPx)
        {
            return top;
        }

        return default;
    }

    private static PixelRect HitUnlocked(
        int screenX,
        int screenY,
        IReadOnlyList<IntPtr> ignoreHwnds,
        IReadOnlyList<MonitorInfo> monitors)
    {
        PixelRect? uia = TryAutomation(screenX, screenY);
        IntPtr hwnd = CaptureNative.WindowFromPoint(new POINT(screenX, screenY));
        if (hwnd != IntPtr.Zero && Contains(ignoreHwnds, hwnd))
        {
            hwnd = IntPtr.Zero;
        }

        if (hwnd != IntPtr.Zero && IsUsableWindow(hwnd))
        {
            IntPtr deep = DeepestChild(hwnd, screenX, screenY);
            if (deep != IntPtr.Zero && IsUsableWindow(deep) && TryRect(deep, out PixelRect child) && !child.IsEmpty)
            {
                if (uia is PixelRect inner
                    && !inner.IsEmpty
                    && inner.Width * inner.Height <= child.Width * child.Height
                    && inner.Width >= CaptureUx.MinCommitPx
                    && inner.Height >= CaptureUx.MinCommitPx)
                {
                    return inner;
                }

                if (child.Width >= CaptureUx.MinCommitPx && child.Height >= CaptureUx.MinCommitPx)
                {
                    return child;
                }
            }
        }

        if (uia is PixelRect rect
            && !rect.IsEmpty
            && rect.Width >= CaptureUx.MinCommitPx
            && rect.Height >= CaptureUx.MinCommitPx)
        {
            return rect;
        }

        return default;
    }

    private static PixelRect? TryAutomation(int screenX, int screenY)
    {
        try
        {
            AutomationElement? element = AutomationElement.FromPoint(new System.Windows.Point(screenX, screenY));
            if (element is null)
            {
                return null;
            }

            System.Windows.Rect bounds = element.Current.BoundingRectangle;
            if (bounds.IsEmpty || bounds.Width < CaptureUx.MinCommitPx || bounds.Height < CaptureUx.MinCommitPx)
            {
                return null;
            }

            return new PixelRect(
                (int)Math.Round(bounds.X),
                (int)Math.Round(bounds.Y),
                Math.Max(1, (int)Math.Round(bounds.Width)),
                Math.Max(1, (int)Math.Round(bounds.Height)));
        }
        catch
        {
            return null;
        }
    }

    private static IntPtr DeepestChild(IntPtr root, int screenX, int screenY)
    {
        IntPtr current = root;
        for (int i = 0; i < 24; i++)
        {
            var pt = new POINT(screenX, screenY);
            if (!CaptureNative.ScreenToClient(current, ref pt))
            {
                break;
            }

            IntPtr child = CaptureNative.ChildWindowFromPointEx(
                current,
                pt,
                CaptureNative.CwpSkipInvisible | CaptureNative.CwpSkipTransparent);
            if (child == IntPtr.Zero || child == current)
            {
                break;
            }

            current = child;
        }

        return current;
    }

    private static bool TryRect(IntPtr hwnd, out PixelRect rect)
    {
        rect = default;
        if (!CaptureNative.GetWindowRect(hwnd, out Rect native) || native.Right <= native.Left || native.Bottom <= native.Top)
        {
            return false;
        }

        rect = new PixelRect(native.Left, native.Top, native.Right - native.Left, native.Bottom - native.Top);
        return true;
    }

    private static bool IsUsableWindow(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !CaptureNative.IsWindowVisible(hwnd) || IsCloaked(hwnd))
        {
            return false;
        }

        string cls = ClassName(hwnd);
        foreach (string desktop in DesktopClasses)
        {
            if (string.Equals(cls, desktop, StringComparison.Ordinal))
            {
                return false;
            }
        }

        return true;
    }

    private static bool IsCloaked(IntPtr hwnd)
    {
        try
        {
            if (CaptureNative.DwmGetWindowAttribute(hwnd, CaptureNative.DwmwaCloaked, out int cloaked, sizeof(int)) == 0)
            {
                return cloaked != 0;
            }
        }
        catch
        {
        }

        return false;
    }

    private static string ClassName(IntPtr hwnd)
    {
        var buffer = new StringBuilder(256);
        return CaptureNative.GetClassNameW(hwnd, buffer, buffer.Capacity) > 0 ? buffer.ToString() : "";
    }

    public static PixelRect WorkAreaAt(int screenX, int screenY, IReadOnlyList<MonitorInfo> monitors)
    {
        foreach (MonitorInfo monitor in monitors)
        {
            if (monitor.Bounds.Contains(screenX, screenY))
            {
                return monitor.WorkArea.IsEmpty ? monitor.Bounds : monitor.WorkArea;
            }
        }

        return monitors.Count > 0 ? monitors[0].Bounds : new PixelRect(screenX, screenY, 4, 4);
    }

    private static bool Contains(IReadOnlyList<IntPtr> hwnds, IntPtr hwnd)
    {
        IntPtr root = CaptureNative.GetAncestor(hwnd, CaptureNative.GaRoot);
        foreach (IntPtr item in hwnds)
        {
            if (item == hwnd || item == root || (root != IntPtr.Zero && item == root))
            {
                return true;
            }

            // Also skip if hwnd is a descendant of an overlay root.
            if (item != IntPtr.Zero)
            {
                IntPtr walk = hwnd;
                for (int i = 0; i < 8 && walk != IntPtr.Zero; i++)
                {
                    if (walk == item)
                    {
                        return true;
                    }

                    walk = CaptureNative.GetAncestor(walk, CaptureNative.GaParent);
                }
            }
        }

        return false;
    }

    /// <summary>Make overlays click-through for hit-test; do NOT Hide (that killed hover MouseMove).</summary>
    private static void RunThroughOverlays(IReadOnlyList<IntPtr> hwnds, Action action)
    {
        var saved = new List<(IntPtr Hwnd, long Style)>(hwnds.Count);
        foreach (IntPtr hwnd in hwnds)
        {
            if (hwnd == IntPtr.Zero)
            {
                continue;
            }

            long style = CaptureNative.GetWindowLongPtr(hwnd, CaptureNative.GwlpExstyle).ToInt64();
            saved.Add((hwnd, style));
            CaptureNative.SetWindowLongPtr(
                hwnd,
                CaptureNative.GwlpExstyle,
                new IntPtr(style | CaptureNative.WsExTransparent | CaptureNative.WsExLayered));
        }

        try
        {
            action();
        }
        finally
        {
            foreach ((IntPtr hwnd, long style) in saved)
            {
                CaptureNative.SetWindowLongPtr(hwnd, CaptureNative.GwlpExstyle, new IntPtr(style));
            }
        }
    }
}
