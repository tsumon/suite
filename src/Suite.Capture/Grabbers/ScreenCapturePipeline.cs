namespace Suite.Capture.Grabbers;

internal static class ScreenCapturePipeline
{
    // Prefer GDI / DXGI first to avoid Win11 GraphicsCapture yellow-border flash.
    // GraphicsCapture last — for affinity-protected windows when others fail.
    public static MonitorCapture Capture(MonitorInfo monitor)
    {
        try
        {
            PixelBuffer gdi = GdiBitBltGrabber.Capture(monitor);
            if (gdi.Width > 0 && gdi.Height > 0)
            {
                return new MonitorCapture
                {
                    Monitor = monitor,
                    Buffer = gdi,
                    Method = CaptureMethod.GdiBitBlt,
                };
            }

            gdi.ReleasePixels();
        }
        catch
        {
        }

        if (DxgiDuplicationGrabber.TryCapture(monitor, out PixelBuffer? buffer) && buffer is not null)
        {
            return new MonitorCapture
            {
                Monitor = monitor,
                Buffer = buffer,
                Method = CaptureMethod.DxgiDuplication,
            };
        }

        if (GraphicsCaptureGrabber.TryCapture(monitor, out buffer) && buffer is not null)
        {
            return new MonitorCapture
            {
                Monitor = monitor,
                Buffer = buffer,
                Method = CaptureMethod.GraphicsCapture,
            };
        }

        return new MonitorCapture
        {
            Monitor = monitor,
            Buffer = GdiBitBltGrabber.Capture(monitor),
            Method = CaptureMethod.GdiBitBlt,
        };
    }

    public static IReadOnlyList<MonitorCapture> CaptureAll()
    {
        IReadOnlyList<MonitorInfo> monitors = MonitorEnumerator.GetAll();
        if (monitors.Count == 0)
        {
            throw new InvalidOperationException("No monitors to capture.");
        }

        var frames = new List<MonitorCapture>(monitors.Count);
        foreach (MonitorInfo monitor in monitors)
        {
            frames.Add(Capture(monitor));
        }

        return frames;
    }
}
