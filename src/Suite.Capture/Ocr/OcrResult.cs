namespace Suite.Capture.Ocr;

public sealed class OcrResult
{
    public bool Succeeded { get; init; }
    public string Text { get; init; } = "";
    public string? Error { get; init; }

    public static OcrResult Ok(string text) => new() { Succeeded = true, Text = text ?? "" };

    public static OcrResult Fail(string error) => new() { Succeeded = false, Error = error };
}
