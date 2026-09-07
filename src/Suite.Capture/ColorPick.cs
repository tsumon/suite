using System.Globalization;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Suite.Capture;

/// <summary>Sample #RRGGBB from a frozen screen buffer. INTERACTION-P2 §6.</summary>
public static class ColorPick
{
    public const string CopiedFmt = "已复制颜色 {0}。";
    public const string Fail = "取色失败，请再试一次。";

    public static string ToHex(byte r, byte g, byte b) =>
        "#" + r.ToString("X2", CultureInfo.InvariantCulture)
             + g.ToString("X2", CultureInfo.InvariantCulture)
             + b.ToString("X2", CultureInfo.InvariantCulture);

    public static bool TrySample(BitmapSource source, int pixelX, int pixelY, out string hex, out Color color)
    {
        hex = "#000000";
        color = Colors.Black;
        if (source is null || pixelX < 0 || pixelY < 0 || pixelX >= source.PixelWidth || pixelY >= source.PixelHeight)
        {
            return false;
        }

        try
        {
            var cropped = new CroppedBitmap(source, new Int32Rect(pixelX, pixelY, 1, 1));
            var converted = new FormatConvertedBitmap(cropped, PixelFormats.Bgra32, null, 0);
            byte[] px = new byte[4];
            converted.CopyPixels(px, 4, 0);
            byte b = px[0], g = px[1], r = px[2];
            color = Color.FromRgb(r, g, b);
            hex = ToHex(r, g, b);
            return true;
        }
        catch
        {
            return false;
        }
    }

    public static bool TryCopyHex(string hex, out string? error)
    {
        error = null;
        try
        {
            Clipboard.SetText(hex);
            return true;
        }
        catch
        {
            error = Fail;
            return false;
        }
    }
}
