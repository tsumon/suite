namespace Suite.Platform;

/// <summary>
/// Physical pixels → WPF device-independent pixels.
/// DIP = pixels * 96 / DPI. At 96 DPI this is 1:1; at 144 DPI a 144px capture is 96 DIP.
/// </summary>
public static class DipConvert
{
    public const double BaselineDpi = 96.0;

    public static double PixelsToDip(double pixels, uint dpi)
    {
        uint safe = dpi == 0 ? 96u : dpi;
        return pixels * BaselineDpi / safe;
    }

    public static (double Width, double Height) Size(int pixelWidth, int pixelHeight, uint dpiX, uint dpiY) =>
        (PixelsToDip(pixelWidth, dpiX), PixelsToDip(pixelHeight, dpiY));

    public static (double Left, double Top) Origin(int pixelX, int pixelY, uint dpiX, uint dpiY) =>
        (PixelsToDip(pixelX, dpiX), PixelsToDip(pixelY, dpiY));
}
