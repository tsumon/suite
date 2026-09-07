namespace Suite.Capture.Ocr;

/// <summary>Scrolling long-shot — design/OCR-SCROLL-SPEC.md v1.1. Best-effort stitch.</summary>
public interface IScrollCaptureService
{
    Task<ScrollCaptureResult> CaptureAsync(
        IntPtr hwnd,
        ScrollCaptureOptions options,
        CancellationToken cancellationToken = default);
}

public sealed class ScrollCaptureOptions
{
    public int MaxHeightPx { get; init; } = 16384;
    public int IdleStopMs { get; init; } = 800;
    public int StepDelayMs { get; init; } = 120;
}

public sealed class ScrollCaptureResult
{
    public bool Succeeded { get; init; }
    public PixelBuffer? Image { get; init; }
    public string? Error { get; init; }

    public static ScrollCaptureResult Ok(PixelBuffer image) => new() { Succeeded = true, Image = image };

    public static ScrollCaptureResult Fail(string error) => new() { Succeeded = false, Error = error };
}
