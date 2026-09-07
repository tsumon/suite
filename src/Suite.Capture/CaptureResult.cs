using System.Windows.Media.Imaging;

namespace Suite.Capture;

public sealed class CaptureRequest
{
    public bool SaveFile { get; init; }
    public string? SaveDirectory { get; init; }
    public bool PinAfterCapture { get; init; }

    /// <summary>INTERACTION-P2 §5. Magnifier during region select.</summary>
    public bool ShowMagnifier { get; init; } = true;
}

public sealed class CaptureResult
{
    public bool Cancelled { get; init; }
    public bool Succeeded { get; init; }
    public bool PinRequested { get; init; }
    public string? Error { get; init; }
    public BitmapSource? Image { get; init; }
    public string? SavedFilePath { get; init; }
    public CaptureMethod Method { get; init; }
    public PixelRect Selection { get; init; }
    public uint DpiX { get; init; } = 96;
    public uint DpiY { get; init; } = 96;
}
