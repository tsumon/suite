using System.ComponentModel;
using System.Runtime.InteropServices;

namespace Suite.Platform;

public static class NativeBroadcast
{
    /// <summary>
    /// Notify apps of ImmersiveColorSet without blocking the caller.
    /// HWND_BROADCAST + SendMessageTimeout waits per top-level window and can freeze the UI for 1–2 minutes.
    /// </summary>
    public static bool SendImmersiveColorSet(out string? error, uint timeoutMs = NativeConstants.ImmersiveColorSetTimeoutMs)
    {
        error = null;
        _ = timeoutMs; // API compat; async notify ignores per-window timeout

        Marshal.SetLastPInvokeError(0);
        if (NativeMethods.SendNotifyMessageW(
                NativeConstants.HwndBroadcast,
                NativeConstants.WmSettingChange,
                UIntPtr.Zero,
                NativeConstants.ImmersiveColorSet))
        {
            return true;
        }

        int code = Marshal.GetLastPInvokeError();
        error = code == 0
            ? "SendNotifyMessageW returned false for ImmersiveColorSet."
            : new Win32Exception(code).Message;
        return false;
    }
}
