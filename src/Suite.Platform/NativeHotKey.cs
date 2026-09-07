using System.ComponentModel;
using System.Runtime.InteropServices;
using Suite.Contracts;

namespace Suite.Platform;

public static class NativeHotKey
{
    public static bool TryRegister(IntPtr hwnd, int id, HotkeyBinding binding, out string? error) =>
        TryRegister(hwnd, id, binding, out error, out _);

    public static bool TryRegister(
        IntPtr hwnd,
        int id,
        HotkeyBinding binding,
        out string? error,
        out int win32Error)
    {
        error = null;
        win32Error = 0;
        if (hwnd == IntPtr.Zero)
        {
            error = "Hotkey window handle is zero.";
            win32Error = ErrorCodes.InvalidHandle;
            return false;
        }

        Marshal.SetLastPInvokeError(0);
        if (NativeMethods.RegisterHotKey(hwnd, id, binding.ToNativeModifiers(), (uint)binding.VirtualKey))
        {
            return true;
        }

        win32Error = Marshal.GetLastPInvokeError();
        error = win32Error == 0
            ? "RegisterHotKey failed."
            : new Win32Exception(win32Error).Message;
        return false;
    }

    public static bool TryUnregister(IntPtr hwnd, int id, out string? error)
    {
        error = null;
        if (hwnd == IntPtr.Zero)
        {
            return true;
        }

        Marshal.SetLastPInvokeError(0);
        if (NativeMethods.UnregisterHotKey(hwnd, id))
        {
            return true;
        }

        int code = Marshal.GetLastPInvokeError();
        if (code == 0)
        {
            return true;
        }

        error = new Win32Exception(code).Message;
        return false;
    }

    private static class ErrorCodes
    {
        public const int InvalidHandle = 6;
    }
}
