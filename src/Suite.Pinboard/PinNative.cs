using System.Runtime.InteropServices;

namespace Suite.Pinboard;

public static class PinNative
{
    private const int GwlpExstyle = -20;
    private const int WsExTransparent = 0x00000020;
    private const int WsExLayered = 0x00080000;
    private const uint SwpNomove = 0x0002;
    private const uint SwpNosize = 0x0001;
    private const uint SwpNozorder = 0x0004;
    private const uint SwpFramechanged = 0x0020;
    private const int MonitorDefaultToNearest = 2;
    private const int MdtEffectiveDpi = 0;

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static extern IntPtr GetWindowLongPtr(IntPtr hWnd, int nIndex);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW")]
    private static extern IntPtr SetWindowLongPtr(IntPtr hWnd, int nIndex, IntPtr dwNewLong);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy, uint uFlags);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetCursorPos(out Point pos);

    [DllImport("user32.dll")]
    private static extern uint GetDpiForSystem();

    [DllImport("user32.dll")]
    private static extern IntPtr MonitorFromPoint(Point pt, uint dwFlags);

    [DllImport("shcore.dll")]
    private static extern int GetDpiForMonitor(IntPtr hmonitor, int dpiType, out uint dpiX, out uint dpiY);

    [StructLayout(LayoutKind.Sequential)]
    private struct Point
    {
        public int X;
        public int Y;
    }

    public static uint SystemDpi()
    {
        try
        {
            uint dpi = GetDpiForSystem();
            return dpi == 0 ? 96u : dpi;
        }
        catch
        {
            return 96;
        }
    }

    /// <summary>Effective DPI of the monitor under the cursor (multi-mon safe).</summary>
    public static uint DpiNearCursor()
    {
        try
        {
            if (!GetCursorPos(out Point pos))
            {
                return SystemDpi();
            }

            IntPtr monitor = MonitorFromPoint(pos, MonitorDefaultToNearest);
            if (monitor == IntPtr.Zero)
            {
                return SystemDpi();
            }

            if (GetDpiForMonitor(monitor, MdtEffectiveDpi, out uint x, out _) >= 0 && x != 0)
            {
                return x;
            }
        }
        catch
        {
        }

        return SystemDpi();
    }

    public static (int X, int Y) CursorPos()
    {
        return GetCursorPos(out Point pos) ? (pos.X, pos.Y) : (0, 0);
    }

    public static void SetClickThrough(IntPtr hwnd, bool enabled)
    {
        if (hwnd == IntPtr.Zero)
        {
            return;
        }

        long style = GetWindowLongPtr(hwnd, GwlpExstyle).ToInt64();
        if (enabled)
        {
            style |= WsExTransparent | WsExLayered;
        }
        else
        {
            style &= ~WsExTransparent;
        }

        SetWindowLongPtr(hwnd, GwlpExstyle, new IntPtr(style));
        SetWindowPos(
            hwnd,
            IntPtr.Zero,
            0,
            0,
            0,
            0,
            SwpNomove | SwpNosize | SwpNozorder | SwpFramechanged);
    }
}
