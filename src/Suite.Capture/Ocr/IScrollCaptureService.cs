namespace Suite.Capture.Ocr;

/// <summary>Scrolling long-shot — design/OCR-SCROLL-SPEC.md. User scrolls; Suite only samples + stitches.</summary>
public interface IScrollCaptureService
{
    /// <summary>
    /// Sample <paramref name="screenRect"/> until idle (no visual change for IdleStopMs)
    /// after the stitch has grown, or until cancel / max height / max duration.
    /// Does <b>not</b> inject WM_MOUSEWHEEL / VSCROLL.
    /// </summary>
    Task<ScrollCaptureResult> CaptureAsync(
        PixelRect screenRect,
        ScrollCaptureOptions options,
        CancellationToken cancellationToken = default);
}

public sealed class ScrollCaptureOptions
{
    public int MaxHeightPx { get; init; } = 16384;
    /// <summary>Stop after this many ms with no new content (default 1.5s).</summary>
    public int IdleStopMs { get; init; } = 1500;
    /// <summary>BitBlt sample interval while waiting for user scroll.</summary>
    public int SampleIntervalMs { get; init; } = 200;
    /// <summary>Hard cap so an animated viewport cannot hang forever.</summary>
    public int MaxDurationMs { get; init; } = 120_000;
    /// <summary>If the user never scrolls, fail after this many ms.</summary>
    public int MaxWaitFirstChangeMs { get; init; } = 45_000;
}

public sealed class ScrollCaptureResult
{
    public bool Succeeded { get; init; }
    public PixelBuffer? Image { get; init; }
    public string? Error { get; init; }

    public static ScrollCaptureResult Ok(PixelBuffer image) => new() { Succeeded = true, Image = image };

    public static ScrollCaptureResult Fail(string error) => new() { Succeeded = false, Error = error };
}
