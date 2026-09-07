using System.IO;
using System.Windows;
using System.Windows.Media.Imaging;

namespace Suite.Capture;

/// <summary>
/// Put formats chat apps (Electron/Chromium) and WeChat-class tools actually read.
/// byte[] under "PNG" often fails paste in Grok Bot; MemoryStream + image/png works better.
/// </summary>
public static class ImageClipboard
{
    public static void Copy(PixelBuffer buffer)
    {
        BitmapSource source = buffer.ToBitmapSource();
        if (source.CanFreeze)
        {
            source.Freeze();
        }

        using var png = new MemoryStream();
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(source));
        encoder.Save(png);
        byte[] bytes = png.ToArray();

        var data = new DataObject();
        data.SetImage(source);
        // Chromium / many messengers prefer a seekable PNG stream, not a raw byte[].
        data.SetData("PNG", new MemoryStream(bytes), false);
        data.SetData("image/png", new MemoryStream(bytes), false);
        Clipboard.SetDataObject(data, true);
    }

    public static bool TryGet(out BitmapSource? image)
    {
        image = null;
        try
        {
            if (!Clipboard.ContainsImage())
            {
                return false;
            }

            BitmapSource? source = Clipboard.GetImage();
            if (source is null)
            {
                return false;
            }

            if (source.CanFreeze)
            {
                source.Freeze();
            }

            image = source;
            return true;
        }
        catch
        {
            return false;
        }
    }
}
