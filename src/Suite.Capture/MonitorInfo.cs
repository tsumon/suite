namespace Suite.Capture;

internal sealed class MonitorInfo
{
    public IntPtr Handle { get; init; }
    public PixelRect Bounds { get; init; }
    public PixelRect WorkArea { get; init; }
    public string DeviceName { get; init; } = "";
    public bool IsPrimary { get; init; }
    public uint DpiX { get; init; } = 96;
    public uint DpiY { get; init; } = 96;
}

internal sealed class MonitorCapture
{
    public required MonitorInfo Monitor { get; init; }
    public required PixelBuffer Buffer { get; init; }
    public required CaptureMethod Method { get; init; }
}
