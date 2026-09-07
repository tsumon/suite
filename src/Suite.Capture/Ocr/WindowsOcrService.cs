using System.Runtime.InteropServices.WindowsRuntime;
using Windows.Graphics.Imaging;
using Windows.Media.Ocr;
using Windows.Storage.Streams;

namespace Suite.Capture.Ocr;

/// <summary>
/// WinRT OCR via Windows.Media.Ocr. Prefer user profile languages; fall back to available packs.
/// Pixel path: BGRA8 PixelBuffer → SoftwareBitmap (CreateCopyFromBuffer).
/// </summary>
public sealed class WindowsOcrService : IOcrService
{
    public const string Copied = "已复制文字。";
    public const string NoText = "没识别到文字。";
    public const string NoLanguage = "系统缺少 OCR 语言包，请在 Windows 语言设置里安装后再试。";
    public const string EngineUnavailable = "本机暂时无法识字。";
    public const string EmptyImage = "没识别到文字。";
    public const string Failed = "本机暂时无法识字。";

    public static bool TryCreate(out WindowsOcrService? service)
    {
        service = null;
        try
        {
            if (OcrEngine.AvailableRecognizerLanguages is null
                || OcrEngine.AvailableRecognizerLanguages.Count == 0)
            {
                return false;
            }

            service = new WindowsOcrService();
            return true;
        }
        catch
        {
            return false;
        }
    }

    public async Task<OcrResult> RecognizeAsync(PixelBuffer image, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (image.Width <= 0 || image.Height <= 0 || image.Bgra.Length == 0)
        {
            return OcrResult.Fail(EmptyImage);
        }

        OcrEngine? engine = CreateEngine();
        if (engine is null)
        {
            return OcrResult.Fail(NoLanguage);
        }

        SoftwareBitmap? bitmap = null;
        try
        {
            bitmap = ToSoftwareBitmap(image);
            if (bitmap is null)
            {
                return OcrResult.Fail(Failed);
            }

            Windows.Media.Ocr.OcrResult raw = await engine.RecognizeAsync(bitmap)
                .AsTask(cancellationToken)
                .ConfigureAwait(false);
            string text = (raw.Text ?? "").Trim();
            if (text.Length == 0)
            {
                return OcrResult.Fail(NoText);
            }

            return OcrResult.Ok(text);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception)
        {
            return OcrResult.Fail(Failed);
        }
        finally
        {
            bitmap?.Dispose();
        }
    }

    private static OcrEngine? CreateEngine()
    {
        try
        {
            OcrEngine? fromProfile = OcrEngine.TryCreateFromUserProfileLanguages();
            if (fromProfile is not null)
            {
                return fromProfile;
            }

            foreach (Windows.Globalization.Language language in OcrEngine.AvailableRecognizerLanguages)
            {
                OcrEngine? fromLang = OcrEngine.TryCreateFromLanguage(language);
                if (fromLang is not null)
                {
                    return fromLang;
                }
            }
        }
        catch
        {
        }

        return null;
    }

    /// <summary>BGRA8 tightly packed (or stride-aware copy) into SoftwareBitmap.</summary>
    private static SoftwareBitmap? ToSoftwareBitmap(PixelBuffer image)
    {
        int width = image.Width;
        int height = image.Height;
        int stride = image.Stride;
        byte[] src = image.Bgra;
        byte[] packed;
        if (stride == width * 4)
        {
            packed = src;
        }
        else
        {
            packed = new byte[width * 4 * height];
            for (int y = 0; y < height; y++)
            {
                System.Buffer.BlockCopy(src, y * stride, packed, y * width * 4, width * 4);
            }
        }

        IBuffer buffer = packed.AsBuffer();
        var bitmap = SoftwareBitmap.CreateCopyFromBuffer(
            buffer,
            BitmapPixelFormat.Bgra8,
            width,
            height,
            BitmapAlphaMode.Premultiplied);
        return bitmap;
    }
}
