using System.Runtime.InteropServices;
using Suite.Capture.Native;

namespace Suite.Capture;

internal static class MonitorEnumerator
{
    public static IReadOnlyList<MonitorInfo> GetAll()
    {
        var list = new List<MonitorInfo>(2);
        CaptureNative.MonitorEnumProc proc = (hMonitor, _, _, _) =>
        {
            var info = new MonitorInfoEx
            {
                CbSize = Marshal.SizeOf<MonitorInfoEx>(),
                SzDevice = "",
            };
            if (!CaptureNative.GetMonitorInfoW(hMonitor, ref info))
            {
                return true;
            }

            uint dpiX = 96;
            uint dpiY = 96;
            if (CaptureNative.GetDpiForMonitor(hMonitor, CaptureNative.MdtEffectiveDpi, out uint x, out uint y) >= 0)
            {
                if (x != 0)
                {
                    dpiX = x;
                }

                if (y != 0)
                {
                    dpiY = y;
                }
            }

            var bounds = new PixelRect(
                info.RcMonitor.Left,
                info.RcMonitor.Top,
                info.RcMonitor.Right - info.RcMonitor.Left,
                info.RcMonitor.Bottom - info.RcMonitor.Top);
            var work = new PixelRect(
                info.RcWork.Left,
                info.RcWork.Top,
                info.RcWork.Right - info.RcWork.Left,
                info.RcWork.Bottom - info.RcWork.Top);
            if (bounds.IsEmpty)
            {
                return true;
            }

            list.Add(new MonitorInfo
            {
                Handle = hMonitor,
                Bounds = bounds,
                WorkArea = work.IsEmpty ? bounds : work,
                DeviceName = info.SzDevice ?? "",
                IsPrimary = (info.DwFlags & CaptureNative.MonitorinfofPrimary) != 0,
                DpiX = dpiX,
                DpiY = dpiY,
            });
            return true;
        };

        CaptureNative.EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, proc, IntPtr.Zero);
        GC.KeepAlive(proc);
        if (list.Count == 0)
        {
            int x = CaptureNative.GetSystemMetrics(CaptureNative.SmtoXvirtualscreen);
            int y = CaptureNative.GetSystemMetrics(CaptureNative.SmtoYvirtualscreen);
            int w = CaptureNative.GetSystemMetrics(CaptureNative.SmtoCvirtualscreen);
            int h = CaptureNative.GetSystemMetrics(CaptureNative.SmtoCyvirtualscreen);
            if (w > 0 && h > 0)
            {
                var virtualBounds = new PixelRect(x, y, w, h);
                list.Add(new MonitorInfo
                {
                    Handle = IntPtr.Zero,
                    Bounds = virtualBounds,
                    WorkArea = virtualBounds,
                    DeviceName = "VIRTUAL",
                    IsPrimary = true,
                });
            }
        }

        return list;
    }
}
