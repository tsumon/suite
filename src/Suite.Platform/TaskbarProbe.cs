using System.Text;

namespace Suite.Platform;

/// <summary>
/// Read-only HWND probe. No composition calls, no TAP, no injection symbols.
/// P1 Host uses this (and Native) to decide classic SWCA vs "Win11 path not delivered".
/// </summary>
public static class TaskbarProbe
{
    public const string PrimaryTrayClass = "Shell_TrayWnd";
    public const string SecondaryTrayClass = "Shell_SecondaryTrayWnd";

    /// <summary>
    /// Child class used by the modern XAML taskbar (Win11 default).
    /// Presence means Native should use the TAP path, not SWCA.
    /// </summary>
    public const string XamlBridgeClass = "Windows.UI.Composition.DesktopWindowContentBridge";

    public static TaskbarKind Detect()
    {
        IntPtr primary = TaskbarNative.FindWindowW(PrimaryTrayClass, null);
        if (primary == IntPtr.Zero || !TaskbarNative.IsWindow(primary))
        {
            return TaskbarKind.NotFound;
        }

        if (HasXamlBridge(primary))
        {
            return TaskbarKind.ModernXaml;
        }

        bool anyXaml = false;
        TaskbarNative.EnumWindows((hWnd, _) =>
        {
            if (GetClass(hWnd) == SecondaryTrayClass && HasXamlBridge(hWnd))
            {
                anyXaml = true;
                return false;
            }

            return true;
        }, IntPtr.Zero);

        return anyXaml ? TaskbarKind.ModernXaml : TaskbarKind.ClassicWin32;
    }

    public static IReadOnlyList<IntPtr> FindTrayWindows()
    {
        var list = new List<IntPtr>();
        IntPtr primary = TaskbarNative.FindWindowW(PrimaryTrayClass, null);
        if (primary != IntPtr.Zero && TaskbarNative.IsWindow(primary))
        {
            list.Add(primary);
        }

        TaskbarNative.EnumWindows((hWnd, _) =>
        {
            if (GetClass(hWnd) == SecondaryTrayClass && TaskbarNative.IsWindow(hWnd) && !list.Contains(hWnd))
            {
                list.Add(hWnd);
            }

            return true;
        }, IntPtr.Zero);

        return list;
    }

    public static bool HasXamlBridge(IntPtr tray)
    {
        if (tray == IntPtr.Zero)
        {
            return false;
        }

        IntPtr child = TaskbarNative.FindWindowExW(tray, IntPtr.Zero, XamlBridgeClass, null);
        return child != IntPtr.Zero;
    }

    private static string GetClass(IntPtr hWnd)
    {
        var buffer = new StringBuilder(TaskbarNative.MaxClassName);
        int n = TaskbarNative.GetClassNameW(hWnd, buffer, buffer.Capacity);
        return n <= 0 ? "" : buffer.ToString();
    }
}
