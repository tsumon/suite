namespace Suite.Capture.Grabbers;

internal static class ScreenCapturePipeline
{
    // Order is ADR §3.3. Protected windows (SetWindowDisplayAffinity) may appear black; do not bypass.
    public static MonitorCapture Capture(MonitorInfo monitor)
    {
        if (GraphicsCaptureGrabber.TryCapture(monitor, out PixelBuffer? buffer) && buffer is not null)
        {
            return new MonitorCapture
            {
                Monitor = monitor,
                Buffer = buffer,
                Method = CaptureMethod.GraphicsCapture,
            };
        }

        if (DxgiDuplicationGrabber.TryCapture(monitor, out buffer) && buffer is not null)
        {
            return new MonitorCapture
            {
                Monitor = monitor,
                Buffer = buffer,
                Method = CaptureMethod.DxgiDuplication,
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
