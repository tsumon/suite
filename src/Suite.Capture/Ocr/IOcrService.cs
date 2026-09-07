namespace Suite.Capture.Ocr;

/// <summary>OCR contract — design/OCR-SCROLL-SPEC.md v1.1. Prefer WindowsOcrService on Windows.</summary>
public interface IOcrService
{
    Task<OcrResult> RecognizeAsync(PixelBuffer image, CancellationToken cancellationToken = default);
}
