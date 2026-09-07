using System.Collections.Concurrent;
using System.Text;

namespace Suite.Platform;

/// <summary>
/// Documented Win32: FindWindow("Shell_TrayWnd") + SetParent.
/// Own implementation — not TrafficMonitor source.
/// Win11 centered taskbar / widgets may still overlap; caller must fall back to a floating window.
/// </summary>
public static class TaskbarEmbed
{
    public const int DefaultWidgetWidthPx = 148;
    public const string NotifyClass = "TrayNotifyWnd";

    private static readonly ConcurrentDictionary<nint, nint> OriginalStyles = new();

    public static IntPtr FindPrimaryTray()
    {
        IntPtr tray = TaskbarNative.FindWindowW(TaskbarProbe.PrimaryTrayClass, null);
        return tray != IntPtr.Zero && TaskbarNative.IsWindow(tray) ? tray : IntPtr.Zero;
    }

    /// <summary>Primary tray client size in physical pixels. Height drives embedded widget DIP height.</summary>
    public static bool TryGetTrayClientSize(out int widthPx, out int heightPx)
    {
        widthPx = 0;
        heightPx = 0;
        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || !TaskbarEmbedNative.GetClientRect(tray, out TaskbarEmbedNative.Rect trayRc))
        {
            return false;
        }

        widthPx = trayRc.Right - trayRc.Left;
        heightPx = trayRc.Bottom - trayRc.Top;
        return widthPx >= 32 && heightPx >= 16;
    }

    public static (int X, int Y, int Width, int Height) ComputeSlot(
        int trayClientWidth,
        int trayClientHeight,
        int? notifyLeft,
        int desiredWidth)
    {
        int height = Math.Max(1, trayClientHeight);
        int width = Math.Clamp(desiredWidth, 40, Math.Max(40, trayClientWidth - 8));
        int x;
        if (notifyLeft is int left && left - width - 4 >= 0)
        {
            x = left - width - 4;
        }
        else
        {
            x = Math.Max(0, trayClientWidth - width - 4);
        }

        if (x + width > trayClientWidth)
        {
            x = Math.Max(0, trayClientWidth - width);
        }

        return (x, 0, width, height);
    }

    public static bool SlotOverlapsNotify(int slotX, int slotWidth, int? notifyLeft) =>
        notifyLeft is int left && left >= 0 && slotX + slotWidth > left;


    public static bool TryGetWindowRect(IntPtr hwnd, out int left, out int top, out int right, out int bottom)
    {
        left = top = right = bottom = 0;
        if (hwnd == IntPtr.Zero || !TaskbarEmbedNative.GetWindowRect(hwnd, out TaskbarEmbedNative.Rect rc))
        {
            return false;
        }

        left = rc.Left;
        top = rc.Top;
        right = rc.Right;
        bottom = rc.Bottom;
        return true;
    }

    public static bool IsWindowVisible(IntPtr hwnd) =>
        hwnd != IntPtr.Zero && TaskbarNative.IsWindow(hwnd) && TaskbarNative.IsWindowVisible(hwnd);

    public static bool IsAttached(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero)
        {
            return false;
        }

        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || TaskbarEmbedNative.GetParent(hwnd) != tray)
        {
            return false;
        }

        // Owner relationship (popup) also makes GetParent return the tray — require WS_CHILD for true embed.
        long style = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        return (style & TaskbarEmbedNative.WsChild) != 0;
    }


    /// <summary>Best-effort embed into a specific tray HWND (primary or Shell_SecondaryTrayWnd). INTERACTION-P2 §7.</summary>
    public static bool TryAttachToTray(IntPtr hwnd, IntPtr tray, int widgetWidthPx, out string? error)
    {
        error = null;
        if (hwnd == IntPtr.Zero)
        {
            error = "窗口句柄无效。";
            return false;
        }

        if (tray == IntPtr.Zero || !TaskbarNative.IsWindow(tray))
        {
            error = "找不到任务栏。";
            return false;
        }

        nint original = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        OriginalStyles.TryAdd((nint)hwnd, original);
        long style = original;
        style = (style | TaskbarEmbedNative.WsChild) & ~TaskbarEmbedNative.WsPopup;
        TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, (IntPtr)style);
        TaskbarEmbedNative.SetParent(hwnd, tray);
        if (TaskbarEmbedNative.GetParent(hwnd) != tray)
        {
            RestoreStyle(hwnd);
            error = "系统不允许把网速嵌进去。";
            return false;
        }

        if (!TaskbarEmbedNative.GetClientRect(tray, out TaskbarEmbedNative.Rect trayRc))
        {
            TryDetach(hwnd, out _);
            error = "无法读取任务栏大小。";
            return false;
        }

        int trayW = trayRc.Right - trayRc.Left;
        int trayH = trayRc.Bottom - trayRc.Top;
        int? notifyLeft = null;
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero
            && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc)
            && TaskbarEmbedNative.GetWindowRect(tray, out TaskbarEmbedNative.Rect trayScreen))
        {
            notifyLeft = notifyRc.Left - trayScreen.Left;
        }

        var slot = ComputeSlot(trayW, trayH, notifyLeft, widgetWidthPx);
        if (SlotOverlapsNotify(slot.X, slot.Width, notifyLeft))
        {
            TryDetach(hwnd, out _);
            error = "任务栏布局不允许嵌入。";
            return false;
        }

        // HWND_TOP without SWP_NOZORDER: Win11 XAML bridge otherwise covers the child.
        if (!TaskbarEmbedNative.SetWindowPos(
                hwnd,
                TaskbarEmbedNative.HwndTop,
                slot.X,
                slot.Y,
                slot.Width,
                slot.Height,
                TaskbarEmbedNative.SwpShowwindow | TaskbarEmbedNative.SwpFramechanged))
        {
            TryDetach(hwnd, out _);
            error = "任务栏里没有足够位置。";
            return false;
        }

        ForceVisiblePaint(hwnd);
        if (!IsWindowVisible(hwnd))
        {
            TryDetach(hwnd, out _);
            error = "嵌入后窗口仍不可见。";
            return false;
        }

        return true;
    }

    public static bool TryAttach(IntPtr hwnd, int widgetWidthPx, out string? error)
    {
        error = null;
        if (hwnd == IntPtr.Zero)
        {
            error = "窗口句柄无效。";
            return false;
        }

        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero)
        {
            error = "找不到任务栏。";
            return false;
        }

        nint original = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        OriginalStyles.TryAdd((nint)hwnd, original);
        long style = original;
        style = (style | TaskbarEmbedNative.WsChild) & ~TaskbarEmbedNative.WsPopup;
        TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, (IntPtr)style);

        TaskbarEmbedNative.SetParent(hwnd, tray);
        if (TaskbarEmbedNative.GetParent(hwnd) != tray)
        {
            RestoreStyle(hwnd);
            error = "系统不允许把网速嵌进去。";
            return false;
        }

        if (!TryReposition(hwnd, widgetWidthPx, out error))
        {
            TryDetach(hwnd, out _);
            return false;
        }

        return true;
    }

    public static bool TryReposition(IntPtr hwnd, int widgetWidthPx, out string? error)
    {
        error = null;
        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero)
        {
            error = "找不到任务栏。";
            return false;
        }

        if (!TaskbarEmbedNative.GetClientRect(tray, out TaskbarEmbedNative.Rect trayRc))
        {
            error = "无法读取任务栏大小。";
            return false;
        }

        int trayW = trayRc.Right - trayRc.Left;
        int trayH = trayRc.Bottom - trayRc.Top;
        if (trayW < 32 || trayH < 16)
        {
            error = "无法读取任务栏大小。";
            return false;
        }

        int? notifyLeft = null;
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero
            && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc)
            && TaskbarEmbedNative.GetWindowRect(tray, out TaskbarEmbedNative.Rect trayScreen))
        {
            notifyLeft = notifyRc.Left - trayScreen.Left;
        }

        var slot = ComputeSlot(trayW, trayH, notifyLeft, widgetWidthPx);
        if (SlotOverlapsNotify(slot.X, slot.Width, notifyLeft))
        {
            error = "任务栏布局不允许嵌入。";
            return false;
        }

        if (!TaskbarEmbedNative.SetWindowPos(
                hwnd,
                TaskbarEmbedNative.HwndTop,
                slot.X,
                slot.Y,
                slot.Width,
                slot.Height,
                TaskbarEmbedNative.SwpShowwindow | TaskbarEmbedNative.SwpFramechanged))
        {
            error = "任务栏里没有足够位置。";
            return false;
        }

        ForceVisiblePaint(hwnd);
        return true;
    }

    public static bool TryDetach(IntPtr hwnd, out string? error)
    {
        error = null;
        if (hwnd == IntPtr.Zero)
        {
            error = "窗口句柄无效。";
            return false;
        }

        TaskbarEmbedNative.SetParent(hwnd, IntPtr.Zero);
        TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlpHwndParent, IntPtr.Zero);
        RestoreStyle(hwnd);
        TaskbarEmbedNative.SetWindowPos(
            hwnd,
            IntPtr.Zero,
            0,
            0,
            0,
            0,
            TaskbarEmbedNative.SwpNozorder | TaskbarEmbedNative.SwpFramechanged
            | TaskbarEmbedNative.SwpNosize | TaskbarEmbedNative.SwpNomove);
        return true;
    }

    public static uint DpiFor(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero)
        {
            return 96;
        }

        uint dpi = TaskbarEmbedNative.GetDpiForWindow(hwnd);
        return dpi == 0 ? 96u : dpi;
    }



    public const string XamlBridgeClass = "Windows.UI.Composition.DesktopWindowContentBridge";

    public static bool TryGetPrimaryDockScreenRect(int widgetWidthPx, out int screenX, out int screenY, out int width, out int height)
    {
        screenX = screenY = width = height = 0;
        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            return false;
        }

        int trayW = tr - tl;
        int trayH = tb - tt;
        int? notifyLeft = null;
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc))
        {
            notifyLeft = notifyRc.Left - tl;
        }

        var slot = ComputeSlot(trayW, trayH, notifyLeft, widgetWidthPx);
        screenX = tl + slot.X;
        // Sit just above the tray — Win11 z-band covers in-bar TopMost; WPF must own styles.
        screenY = Math.Max(0, tt - Math.Max(slot.Height, 40));
        width = slot.Width;
        height = Math.Max(slot.Height, 40);
        return width >= 8 && height >= 8;
    }



    /// <summary>Win11 taskbar XAML island covers SetParent children in composition even when z-order is top.</summary>
    public static bool HasWin11XamlBridge(IntPtr tray)
    {
        if (tray == IntPtr.Zero)
        {
            return false;
        }

        IntPtr bridge = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, XamlBridgeClass, null);
        return bridge != IntPtr.Zero && TaskbarNative.IsWindow(bridge);
    }

    /// <summary>
    /// Top-most tool window docked over the primary tray slot (left of TrayNotifyWnd).
    /// Used when Win11 XAML bridge would hide a SetParent child. Not a desktop float — locked to the tray bar.
    /// </summary>
    public static bool TryDockOverPrimaryTray(IntPtr hwnd, int widgetWidthPx, out string? error)
    {
        error = null;
        if (hwnd == IntPtr.Zero)
        {
            error = "窗口句柄无效。";
            return false;
        }

        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            error = "找不到任务栏。";
            return false;
        }

        // Detach from tray if a previous SetParent left us as WS_CHILD — but do NOT
        // SetWindowLong style bits on a WPF HWND (breaks DWM presentation → blank chip).
        long style = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        if ((style & TaskbarEmbedNative.WsChild) != 0 || TaskbarEmbedNative.GetParent(hwnd) == tray)
        {
            TaskbarEmbedNative.SetParent(hwnd, IntPtr.Zero);
            if (OriginalStyles.TryGetValue((nint)hwnd, out nint original))
            {
                TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, original);
            }
            else
            {
                // Restore popup from child without inventing other bits.
                style = (style | TaskbarEmbedNative.WsPopup | TaskbarEmbedNative.WsVisible) & ~TaskbarEmbedNative.WsChild;
                TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, (IntPtr)style);
            }
        }

        int trayW = tr - tl;
        int trayH = tb - tt;
        int? notifyLeft = null;
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc))
        {
            notifyLeft = notifyRc.Left - tl;
        }

        var slot = ComputeSlot(trayW, trayH, notifyLeft, widgetWidthPx);
        int screenX = tl + slot.X;
        // Win11 taskbar z-band paints over normal TopMost windows that sit IN the bar.
        // Dock immediately above the tray edge so rates stay visible next to the clock.
        int screenY = Math.Max(0, tt - Math.Max(slot.Height, 40));
        if (!TaskbarEmbedNative.SetWindowPos(
                hwnd,
                TaskbarEmbedNative.HwndTopmost,
                screenX,
                screenY,
                slot.Width,
                slot.Height,
                TaskbarEmbedNative.SwpShowwindow | TaskbarEmbedNative.SwpFramechanged))
        {
            error = "任务栏里没有足够位置。";
            return false;
        }

        TaskbarEmbedNative.ShowWindow(hwnd, TaskbarEmbedNative.SwShow);
        ForceVisiblePaint(hwnd);
        if (!IsDockedOverPrimaryTray(hwnd))
        {
            error = "停靠任务栏后仍不可见。";
            return false;
        }

        return true;
    }

    /// <summary>
    /// Top-level TopMost layered-capable window positioned IN the tray slot (left of notify).
    /// No SetParent — UpdateLayeredWindow works reliably; WS_CHILD of Shell_TrayWnd does not.
    /// </summary>
    public static bool TryDockInTraySlot(IntPtr hwnd, IntPtr tray, int widgetWidthPx, out string? error)
    {
        error = null;
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            error = "窗口句柄无效。";
            return false;
        }

        if (tray == IntPtr.Zero || !TaskbarNative.IsWindow(tray)
            || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            error = "找不到任务栏。";
            return false;
        }

        // Already sitting on the slot — do not SetWindowPos/ShowWindow (gray flash).
        if (IsInTraySlot(hwnd, tray, widgetWidthPx))
        {
            return true;
        }

        // Ensure we are NOT a child of the tray — ULW is unreliable on WS_CHILD.
        long style = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        if ((style & TaskbarEmbedNative.WsChild) != 0 || TaskbarEmbedNative.GetParent(hwnd) == tray)
        {
            TaskbarEmbedNative.SetParent(hwnd, IntPtr.Zero);
            if (OriginalStyles.TryGetValue((nint)hwnd, out nint original))
            {
                TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, original);
            }
            else
            {
                style = (style | TaskbarEmbedNative.WsPopup | TaskbarEmbedNative.WsVisible) & ~TaskbarEmbedNative.WsChild;
                TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, (IntPtr)style);
            }
        }

        // TopMost + ToolWindow stay on CreateParams; reinforce TopMost here.
        long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        TaskbarEmbedNative.SetWindowLongPtr(
            hwnd,
            TaskbarEmbedNative.GwlExstyle,
            (IntPtr)((ex | TaskbarEmbedNative.WsExTopmost | TaskbarEmbedNative.WsExToolwindow | TaskbarEmbedNative.WsExLayered)
                     & ~TaskbarEmbedNative.WsExTransparent));

        int trayW = tr - tl;
        int trayH = tb - tt;
        int? notifyLeft = null;
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc))
        {
            notifyLeft = notifyRc.Left - tl;
        }

        var slot = ComputeSlot(trayW, trayH, notifyLeft, widgetWidthPx);
        int screenX = tl + slot.X;
        int screenY = tt; // IN the bar — Joe wants chip on the taskbar, not floating above.
        if (!TaskbarEmbedNative.SetWindowPos(
                hwnd,
                TaskbarEmbedNative.HwndTopmost,
                screenX,
                screenY,
                slot.Width,
                slot.Height,
                TaskbarEmbedNative.SwpShowwindow | TaskbarEmbedNative.SwpNoactivate))
        {
            error = "任务栏里没有足够位置。";
            return false;
        }

        // Do not ShowWindow again — SetWindowPos already showed; second Show flashes gray.
        if (!TryGetWindowRect(hwnd, out int left, out int top, out int right, out int bottom))
        {
            error = "停靠任务栏后仍不可见。";
            return false;
        }

        int w = right - left;
        int h = bottom - top;
        if (w < 8 || h < 8 || !TaskbarNative.IsWindowVisible(hwnd))
        {
            error = "停靠任务栏后仍不可见。";
            return false;
        }

        return true;
    }

    /// <summary>
    /// True when hwnd is already a top-level (non-child) window sitting on the expected tray slot
    /// within a few pixels — heartbeat must NOT re-dock in this case.
    /// </summary>
    public static bool IsInTraySlot(IntPtr hwnd, IntPtr tray, int widgetWidthPx, int slackPx = 4)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd) || !TaskbarNative.IsWindowVisible(hwnd))
        {
            return false;
        }

        if (tray == IntPtr.Zero || !TaskbarNative.IsWindow(tray)
            || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            return false;
        }

        // Must not be WS_CHILD of tray (ULW path is top-level dock).
        if (IsAttached(hwnd) || TaskbarEmbedNative.GetParent(hwnd) == tray)
        {
            return false;
        }

        long style = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        if ((style & TaskbarEmbedNative.WsChild) != 0)
        {
            return false;
        }

        int trayW = tr - tl;
        int trayH = tb - tt;
        int? notifyLeft = null;
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc))
        {
            notifyLeft = notifyRc.Left - tl;
        }

        var slot = ComputeSlot(trayW, trayH, notifyLeft, widgetWidthPx);
        int expectX = tl + slot.X;
        int expectY = tt;
        if (!TryGetWindowRect(hwnd, out int left, out int top, out int right, out int bottom))
        {
            return false;
        }

        int w = right - left;
        int h = bottom - top;
        if (w < 8 || h < 8)
        {
            return false;
        }

        return Math.Abs(left - expectX) <= slackPx
            && Math.Abs(top - expectY) <= slackPx
            && Math.Abs(w - slot.Width) <= slackPx + 2
            && Math.Abs(h - slot.Height) <= slackPx + 2;
    }

    public static bool TryDockInPrimaryTraySlot(IntPtr hwnd, int widgetWidthPx, out string? error) =>
        TryDockInTraySlot(hwnd, FindPrimaryTray(), widgetWidthPx, out error);

    public static bool IsDockedOverPrimaryTray(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd) || !TaskbarNative.IsWindowVisible(hwnd))
        {
            return false;
        }

        // Docked = not a WS_CHILD of the tray, but rect overlaps/abuts the tray bar.
        if (IsAttached(hwnd))
        {
            return false;
        }

        if (!TryGetWindowRect(hwnd, out int left, out int top, out int right, out int bottom))
        {
            return false;
        }

        int w = right - left;
        int h = bottom - top;
        if (w < 8 || h < 8)
        {
            return false;
        }

        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            return false;
        }

        bool horizontallyOverTray = left < tr && right > tl;
        // Fully above but touching, or overlapping the bar.
        bool verticallyNearTray = bottom >= tt - 4 && top <= tb + 4;
        return horizontallyOverTray && verticallyNearTray;
    }



    public const string NetSpeedTrayTitle = "Suite NetSpeed Tray";
    public const string NetSpeedFloatTitle = "Suite NetSpeed";

    /// <summary>
    /// Destroy a HWND we own (detach from tray first). Safe no-op if already gone.
    /// Child-of-Shell_TrayWnd must be destroyed explicitly — process exit alone can leave ghosts.
    /// </summary>
    public static void DestroyHwnd(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            return;
        }

        try
        {
            ClearChromaKeyTransparency(hwnd);
        }
        catch
        {
        }

        try
        {
            TryDetach(hwnd, out _);
        }
        catch
        {
        }

        if (TaskbarNative.IsWindow(hwnd))
        {
            TaskbarNative.DestroyWindow(hwnd);
        }
    }

    /// <summary>
    /// On startup (and before re-embed): destroy leftover Suite net-speed HWNDs under every tray
    /// from a previous process that exited without DestroyWindow.
    /// </summary>
    /// <summary>
    /// Destroy leftover Suite net-speed HWNDs under every tray (previous process exit without DestroyWindow).
    /// Call on startup BEFORE creating a new embed widget. Optionally skip HWNDs owned by keepPid.
    /// </summary>
    public static int CleanupOrphanNetSpeedWindows(int? keepPid = null)
    {
        int destroyed = 0;
        foreach (IntPtr tray in TaskbarProbe.FindTrayWindows())
        {
            if (tray == IntPtr.Zero)
            {
                continue;
            }

            var doomed = new List<IntPtr>();
            TaskbarNative.EnumChildWindows(
                tray,
                (hWnd, _) =>
                {
                    if (hWnd == IntPtr.Zero || !TaskbarNative.IsWindow(hWnd))
                    {
                        return true;
                    }

                    TaskbarNative.GetWindowThreadProcessId(hWnd, out uint pid);
                    if (keepPid is int keep && pid == (uint)keep)
                    {
                        return true;
                    }

                    string title = GetWindowTitle(hWnd);
                    if (!IsNetSpeedWindowTitle(title))
                    {
                        return true;
                    }

                    doomed.Add(hWnd);
                    return true;
                },
                IntPtr.Zero);

            foreach (IntPtr h in doomed)
            {
                DestroyHwnd(h);
                destroyed++;
            }
        }

        return destroyed;
    }

    public static bool IsNetSpeedWindowTitle(string? title) =>
        !string.IsNullOrEmpty(title)
        && (title.Equals(NetSpeedTrayTitle, StringComparison.Ordinal)
            || title.Equals(NetSpeedFloatTitle, StringComparison.Ordinal)
            || title.StartsWith("Suite NetSpeed", StringComparison.Ordinal));

    private static string GetWindowTitle(IntPtr hwnd)
    {
        var sb = new StringBuilder(256);
        _ = TaskbarNative.GetWindowTextW(hwnd, sb, sb.Capacity);
        return sb.ToString();
    }



    /// <summary>
    /// TrafficMonitor-style clear taskbar slot: magenta chroma-key (no WPF AllowsTransparency).
    /// Returns true only when WS_EX_LAYERED stuck and LWA_COLORKEY is active — otherwise magenta would show.
    /// </summary>
    public static bool ApplyChromaKeyTransparency(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            return false;
        }

        // Apply twice: WPF / SetWindowPos often clear WS_EX_LAYERED on the next layout tick.
        for (int attempt = 0; attempt < 2; attempt++)
        {
            long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
            ex = (ex | TaskbarEmbedNative.WsExLayered) & ~TaskbarEmbedNative.WsExTransparent;
            TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle, (IntPtr)ex);
            TaskbarEmbedNative.SetLayeredWindowAttributes(
                hwnd,
                TaskbarEmbedNative.ColorKeyMagenta,
                0,
                TaskbarEmbedNative.LwaColorkey);
        }

        return IsChromaKeyActive(hwnd);
    }

    /// <summary>True when layered color-key is actually in effect (magenta pixels will not paint).</summary>
    public static bool IsChromaKeyActive(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            return false;
        }

        long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        if ((ex & TaskbarEmbedNative.WsExLayered) == 0)
        {
            return false;
        }

        if (!TaskbarEmbedNative.GetLayeredWindowAttributes(hwnd, out uint key, out _, out uint flags))
        {
            return false;
        }

        return (flags & TaskbarEmbedNative.LwaColorkey) != 0
            && key == TaskbarEmbedNative.ColorKeyMagenta;
    }

    public static void ClearChromaKeyTransparency(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            return;
        }

        long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        if ((ex & TaskbarEmbedNative.WsExLayered) == 0)
        {
            return;
        }

        // Drop LAYERED entirely when leaving chroma mode — avoids LWA_ALPHA fighting future COLORKEY.
        TaskbarEmbedNative.SetWindowLongPtr(
            hwnd,
            TaskbarEmbedNative.GwlExstyle,
            (IntPtr)(ex & ~TaskbarEmbedNative.WsExLayered));
    }

    /// <summary>
    /// Sample a tray pixel just outside the widget slot (left of notify / mid-bar).
    /// COLORREF is 0x00BBGGRR — convert to WPF-friendly 0xAARRGGBB.
    /// </summary>
    public static bool TrySampleTrayArgb(out uint argb)
    {
        argb = 0xFF2B2B2B;
        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            return false;
        }

        int trayH = Math.Max(1, tb - tt);
        int trayW = Math.Max(1, tr - tl);
        // Prefer a pixel left of the notify area, vertically centered — usually solid taskbar chrome.
        int sampleX = tl + Math.Max(8, trayW / 3);
        IntPtr notify = TaskbarEmbedNative.FindWindowExW(tray, IntPtr.Zero, NotifyClass, null);
        if (notify != IntPtr.Zero && TaskbarEmbedNative.GetWindowRect(notify, out TaskbarEmbedNative.Rect notifyRc))
        {
            sampleX = Math.Max(tl + 4, notifyRc.Left - 24);
        }

        int sampleY = tt + trayH / 2;
        IntPtr hdc = TaskbarEmbedNative.GetDC(IntPtr.Zero);
        if (hdc == IntPtr.Zero)
        {
            return false;
        }

        try
        {
            uint colorRef = TaskbarEmbedNative.GetPixel(hdc, sampleX, sampleY);
            if (colorRef == 0xFFFFFFFF)
            {
                return false;
            }

            byte r = (byte)(colorRef & 0xFF);
            byte g = (byte)((colorRef >> 8) & 0xFF);
            byte b = (byte)((colorRef >> 16) & 0xFF);
            // Reject pure magenta (would mean we sampled our own key fill).
            if (r == 0xFF && g == 0x00 && b == 0xFF)
            {
                return false;
            }

            argb = 0xFF000000u | ((uint)r << 16) | ((uint)g << 8) | b;
            return true;
        }
        finally
        {
            _ = TaskbarEmbedNative.ReleaseDC(IntPtr.Zero, hdc);
        }
    }

    /// <summary>Opaque dark fallback when chroma-key cannot stick — never leave magenta on screen.</summary>
    public const uint FallbackOpaqueArgb = 0xFF2B2B2B;

    /// <summary>
    /// After SetParent into Shell_TrayWnd, WPF children often stay "attached" but not painted.
    /// Force WS_VISIBLE, z-order above the XAML bridge, and an immediate redraw.
    /// Does NOT touch SetLayeredWindowAttributes — LWA_ALPHA here used to wipe LWA_COLORKEY and
    /// flash magenta (#FF00FF) on the taskbar.
    /// </summary>
    public static void ForceVisiblePaint(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            return;
        }

        long style = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        if ((style & TaskbarEmbedNative.WsVisible) == 0)
        {
            TaskbarEmbedNative.SetWindowLongPtr(
                hwnd,
                TaskbarEmbedNative.GwlStyle,
                (IntPtr)(style | TaskbarEmbedNative.WsVisible));
        }

        long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        // Never leave WS_EX_TRANSPARENT hit-test pass-through on the rate chip.
        if ((ex & TaskbarEmbedNative.WsExTransparent) != 0)
        {
            TaskbarEmbedNative.SetWindowLongPtr(
                hwnd,
                TaskbarEmbedNative.GwlExstyle,
                (IntPtr)(ex & ~TaskbarEmbedNative.WsExTransparent));
        }

        // Intentionally do NOT call SetLayeredWindowAttributes(LWA_ALPHA) here.
        // That overwrote LWA_COLORKEY and made the chroma-key magenta fill visible.

        TaskbarEmbedNative.ShowWindow(hwnd, TaskbarEmbedNative.SwShow);
        TaskbarEmbedNative.SetWindowPos(
            hwnd,
            TaskbarEmbedNative.HwndTop,
            0,
            0,
            0,
            0,
            TaskbarEmbedNative.SwpNomove | TaskbarEmbedNative.SwpNosize
            | TaskbarEmbedNative.SwpShowwindow | TaskbarEmbedNative.SwpFramechanged);
        TaskbarEmbedNative.InvalidateRect(hwnd, IntPtr.Zero, true);
        TaskbarEmbedNative.RedrawWindow(
            hwnd,
            IntPtr.Zero,
            IntPtr.Zero,
            TaskbarEmbedNative.RdwInvalidate
            | TaskbarEmbedNative.RdwErase
            | TaskbarEmbedNative.RdwFrame
            | TaskbarEmbedNative.RdwAllchildren
            | TaskbarEmbedNative.RdwUpdatenow);
    }

    /// <summary>Child embed or Win11 tray-dock: visible with non-trivial rect overlapping the tray bar.</summary>
    public static bool IsProvenVisibleOnPrimaryTray(IntPtr hwnd)
    {
        if (IsDockedOverPrimaryTray(hwnd))
        {
            return true;
        }

        if (!IsAttached(hwnd) || !IsWindowVisible(hwnd))
        {
            return false;
        }

        if (!TryGetWindowRect(hwnd, out int left, out int top, out int right, out int bottom))
        {
            return false;
        }

        int w = right - left;
        int h = bottom - top;
        if (w < 8 || h < 8)
        {
            return false;
        }

        IntPtr tray = FindPrimaryTray();
        if (tray == IntPtr.Zero || !TryGetWindowRect(tray, out int tl, out int tt, out int tr, out int tb))
        {
            return false;
        }

        return left < tr && right > tl && top < tb && bottom > tt;
    }

    private static void RestoreStyle(IntPtr hwnd)
    {
        if (OriginalStyles.TryRemove((nint)hwnd, out nint original))
        {
            TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, original);
            return;
        }

        long style = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle);
        style = (style | TaskbarEmbedNative.WsPopup) & ~TaskbarEmbedNative.WsChild;
        TaskbarEmbedNative.SetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlStyle, (IntPtr)style);
    }

    /// <summary>
    /// Reset WS_EX_LAYERED so a prior SetLayeredWindowAttributes(COLORKEY) does not block
    /// UpdateLayeredWindow. Must be called before the first per-pixel push.
    /// </summary>
    public static void PreparePerPixelLayered(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd))
        {
            return;
        }

        long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        TaskbarEmbedNative.SetWindowLongPtr(
            hwnd,
            TaskbarEmbedNative.GwlExstyle,
            (IntPtr)(ex & ~TaskbarEmbedNative.WsExLayered & ~TaskbarEmbedNative.WsExTransparent));
        ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        TaskbarEmbedNative.SetWindowLongPtr(
            hwnd,
            TaskbarEmbedNative.GwlExstyle,
            (IntPtr)(ex | TaskbarEmbedNative.WsExLayered));
    }

    /// <summary>
    /// Push a full 32bpp premultiplied-ARGB frame via UpdateLayeredWindow (ULW_ALPHA).
    /// Replaces the entire layered surface — no COLORKEY, no ghost glyphs, true per-pixel alpha.
    /// <paramref name="bgraPremul"/> length must be width*height*4, top-down, premultiplied BGRA.
    /// </summary>
    public static bool TryPushPerPixelLayered(IntPtr hwnd, int width, int height, byte[] bgraPremul)
    {
        if (hwnd == IntPtr.Zero || !TaskbarNative.IsWindow(hwnd) || bgraPremul is null)
        {
            return false;
        }

        if (width < 1 || height < 1 || bgraPremul.Length < width * height * 4)
        {
            return false;
        }

        long ex = TaskbarEmbedNative.GetWindowLongPtr(hwnd, TaskbarEmbedNative.GwlExstyle);
        // Only reset LAYERED when missing or COLORKEY attrs still stuck.
        // Do NOT prep on every call — toggling LAYERED flashes WinForms BackColor (black card).
        bool needsPrep = (ex & TaskbarEmbedNative.WsExLayered) == 0;
        if (!needsPrep
            && TaskbarEmbedNative.GetLayeredWindowAttributes(hwnd, out _, out _, out uint flags)
            && (flags & TaskbarEmbedNative.LwaColorkey) != 0)
        {
            needsPrep = true;
        }

        if (needsPrep)
        {
            PreparePerPixelLayered(hwnd);
        }

        IntPtr screenDc = TaskbarEmbedNative.GetDC(IntPtr.Zero);
        if (screenDc == IntPtr.Zero)
        {
            return false;
        }

        IntPtr memDc = TaskbarEmbedNative.CreateCompatibleDC(screenDc);
        if (memDc == IntPtr.Zero)
        {
            _ = TaskbarEmbedNative.ReleaseDC(IntPtr.Zero, screenDc);
            return false;
        }

        var bmi = new TaskbarEmbedNative.BITMAPINFO
        {
            BmiHeader = new TaskbarEmbedNative.BITMAPINFOHEADER
            {
                BiSize = (uint)System.Runtime.InteropServices.Marshal.SizeOf<TaskbarEmbedNative.BITMAPINFOHEADER>(),
                BiWidth = width,
                BiHeight = -height,
                BiPlanes = 1,
                BiBitCount = 32,
                BiCompression = TaskbarEmbedNative.BiRgb,
            },
        };

        IntPtr bits;
        IntPtr dib = TaskbarEmbedNative.CreateDIBSection(
            memDc, ref bmi, TaskbarEmbedNative.DibRgbColors, out bits, IntPtr.Zero, 0);
        if (dib == IntPtr.Zero || bits == IntPtr.Zero)
        {
            _ = TaskbarEmbedNative.DeleteDC(memDc);
            _ = TaskbarEmbedNative.ReleaseDC(IntPtr.Zero, screenDc);
            return false;
        }

        IntPtr old = TaskbarEmbedNative.SelectObject(memDc, dib);
        try
        {
            System.Runtime.InteropServices.Marshal.Copy(bgraPremul, 0, bits, width * height * 4);

            var size = new TaskbarEmbedNative.SIZE { Cx = width, Cy = height };
            var src = new TaskbarEmbedNative.POINT { X = 0, Y = 0 };
            var blend = new TaskbarEmbedNative.BLENDFUNCTION
            {
                BlendOp = TaskbarEmbedNative.AcSrcOver,
                BlendFlags = 0,
                SourceConstantAlpha = 255,
                AlphaFormat = TaskbarEmbedNative.AcSrcAlpha,
            };

            // Pure bitmap swap — no SetWindowPos here (even NOMOVE TopMost caused acrylic flash on Joe).
            return TaskbarEmbedNative.UpdateLayeredWindow(
                hwnd,
                IntPtr.Zero,
                IntPtr.Zero,
                ref size,
                memDc,
                ref src,
                0,
                ref blend,
                TaskbarEmbedNative.UlwAlpha);
        }
        finally
        {
            TaskbarEmbedNative.SelectObject(memDc, old);
            _ = TaskbarEmbedNative.DeleteObject(dib);
            _ = TaskbarEmbedNative.DeleteDC(memDc);
            _ = TaskbarEmbedNative.ReleaseDC(IntPtr.Zero, screenDc);
        }
    }
}
