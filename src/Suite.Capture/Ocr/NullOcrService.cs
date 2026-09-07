namespace Suite.Capture.Ocr;

/// <summary>Stub until Windows.Media.Ocr wiring is verified on tsumon.</summary>
public sealed class NullOcrService : IOcrService
{
    public const string NotEnabled = "识字尚未在本机启用。";

    public Task<OcrResult> RecognizeAsync(PixelBuffer image, CancellationToken cancellationToken = default)
    {
        _ = image;
        cancellationToken.ThrowIfCancellationRequested();
        return Task.FromResult(OcrResult.Fail(NotEnabled));
    }
}
