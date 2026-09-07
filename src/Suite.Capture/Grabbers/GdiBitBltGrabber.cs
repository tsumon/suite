using Suite.Capture.Native;

namespace Suite.Capture.Grabbers;

internal static class GdiBitBltGrabber
{
    public static PixelBuffer Capture(MonitorInfo monitor)
    {
        IntPtr hdcScreen = CaptureNative.GetDC(IntPtr.Zero);
        if (hdcScreen == IntPtr.Zero)
        {
            throw new InvalidOperationException("GetDC failed.");
        }

        IntPtr hdcMem = IntPtr.Zero;
        IntPtr hbmp = IntPtr.Zero;
        IntPtr old = IntPtr.Zero;
        try
        {
            int width = monitor.Bounds.Width;
            int height = monitor.Bounds.Height;
            hdcMem = CaptureNative.CreateCompatibleDC(hdcScreen);
            hbmp = CaptureNative.CreateCompatibleBitmap(hdcScreen, width, height);
            if (hdcMem == IntPtr.Zero || hbmp == IntPtr.Zero)
            {
                throw new InvalidOperationException("CreateCompatibleDC/Bitmap failed.");
            }

            old = CaptureNative.SelectObject(hdcMem, hbmp);
            if (!CaptureNative.BitBlt(
                    hdcMem,
                    0,
                    0,
                    width,
                    height,
                    hdcScreen,
                    monitor.Bounds.X,
                    monitor.Bounds.Y,
                    CaptureNative.Srccopy))
            {
                throw new InvalidOperationException("BitBlt failed.");
            }

            var info = new BitmapInfo
            {
                BmiHeader = new BitmapInfoHeader
                {
                    BiSize = (uint)System.Runtime.InteropServices.Marshal.SizeOf<BitmapInfoHeader>(),
                    BiWidth = width,
                    BiHeight = -height,
                    BiPlanes = 1,
                    BiBitCount = 32,
                    BiCompression = CaptureNative.BiRgb,
                },
            };

            var pixels = new byte[width * 4 * height];
            int lines = CaptureNative.GetDIBits(
                hdcMem,
                hbmp,
                0,
                (uint)height,
                pixels,
                ref info,
                CaptureNative.DibRgbColors);
            if (lines == 0)
            {
                throw new InvalidOperationException("GetDIBits failed.");
            }

            ForceOpaque(pixels);
            return new PixelBuffer(width, height, pixels, width * 4);
        }
        finally
        {
            if (old != IntPtr.Zero && hdcMem != IntPtr.Zero)
            {
                CaptureNative.SelectObject(hdcMem, old);
            }

            if (hbmp != IntPtr.Zero)
            {
                CaptureNative.DeleteObject(hbmp);
            }

            if (hdcMem != IntPtr.Zero)
            {
                CaptureNative.DeleteDC(hdcMem);
            }

            CaptureNative.ReleaseDC(IntPtr.Zero, hdcScreen);
        }
    }

    private static void ForceOpaque(byte[] bgra)
    {
        for (int i = 3; i < bgra.Length; i += 4)
        {
            bgra[i] = 255;
        }
    }
}
