using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Windows.Graphics.Imaging;
using Windows.Media.Ocr;
using Windows.Storage.Streams;

namespace Suite.Capture.Ocr;

/// <summary>
/// WinRT OCR via Windows.Media.Ocr.
/// Prefer AvailableRecognizerLanguages (zh-Hans / zh first); also try user profile languages.
/// Pixel path: BGRA8 PixelBuffer → SoftwareBitmap with Ignore alpha (raw capture is not premultiplied).
/// </summary>
public sealed class WindowsOcrService : IOcrService
{
    public const string Copied = "已复制文字。";
    public const string NoText = "没识别到文字。";
    public const string NoLanguage = "系统缺少 OCR 语言包，请在 Windows 语言设置里安装后再试。";
    public const string EngineUnavailable = "本机暂时无法识字。";
    public const string EmptyImage = "没识别到文字。";
    public const string Failed = "本机暂时无法识字。";
    public const string ClipboardFailed = "文字已识别，但没法写入剪贴板，请再试一次。";

    private readonly string _languageTag;

    private WindowsOcrService(string languageTag)
    {
        _languageTag = languageTag;
    }

    /// <summary>Language tag used by the live engine (e.g. zh-Hans-CN), or empty if unavailable.</summary>
    public string LanguageTag => _languageTag;

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

            if (!TryCreateEngine(out OcrEngine? engine, out string tag) || engine is null)
            {
                return false;
            }

            // Keep engine only long enough to prove create works; Recognize recreates via tag preference.
            _ = engine;
            service = new WindowsOcrService(tag);
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

        if (!TryCreateEngine(out OcrEngine? engine, out _) || engine is null)
        {
            return OcrResult.Fail(
                OcrEngine.AvailableRecognizerLanguages is null
                || OcrEngine.AvailableRecognizerLanguages.Count == 0
                    ? NoLanguage
                    : EngineUnavailable);
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

    /// <summary>
    /// Prefer AvailableRecognizerLanguages (zh-Hans* → zh-* → others), then user profile languages.
    /// </summary>
    internal static bool TryCreateEngine(out OcrEngine? engine, out string languageTag)
    {
        engine = null;
        languageTag = "";
        try
        {
            if (TryCreateFromAvailable(out engine, out languageTag) && engine is not null)
            {
                return true;
            }

            OcrEngine? fromProfile = OcrEngine.TryCreateFromUserProfileLanguages();
            if (fromProfile is not null)
            {
                engine = fromProfile;
                languageTag = fromProfile.RecognizerLanguage?.LanguageTag ?? "profile";
                return true;
            }
        }
        catch
        {
        }

        engine = null;
        languageTag = "";
        return false;
    }

    private static bool TryCreateFromAvailable(out OcrEngine? engine, out string languageTag)
    {
        engine = null;
        languageTag = "";
        var langs = OcrEngine.AvailableRecognizerLanguages;
        if (langs is null || langs.Count == 0)
        {
            return false;
        }

        foreach (Windows.Globalization.Language language in OrderLanguages(langs))
        {
            try
            {
                OcrEngine? created = OcrEngine.TryCreateFromLanguage(language);
                if (created is not null)
                {
                    engine = created;
                    languageTag = language.LanguageTag ?? "";
                    return true;
                }
            }
            catch
            {
            }
        }

        return false;
    }

    /// <summary>zh-Hans* first, then other zh-*, then remaining packs.</summary>
    private static IEnumerable<Windows.Globalization.Language> OrderLanguages(
        IEnumerable<Windows.Globalization.Language> langs)
    {
        static int Rank(string? tag)
        {
            if (string.IsNullOrEmpty(tag))
            {
                return 3;
            }

            if (tag.StartsWith("zh-Hans", StringComparison.OrdinalIgnoreCase))
            {
                return 0;
            }

            if (tag.StartsWith("zh", StringComparison.OrdinalIgnoreCase))
            {
                return 1;
            }

            return 2;
        }

        return langs.OrderBy(l => Rank(l.LanguageTag));
    }

    /// <summary>BGRA8 tightly packed into SoftwareBitmap. Use Ignore — capture buffers are not premultiplied.</summary>
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

        // Screen capture BGRA is typically opaque with straight alpha; Premultiplied mis-interprets it and can blank OCR.
        IBuffer buffer = packed.AsBuffer();
        var bitmap = SoftwareBitmap.CreateCopyFromBuffer(
            buffer,
            BitmapPixelFormat.Bgra8,
            width,
            height,
            BitmapAlphaMode.Ignore);
        return bitmap;
    }
}
