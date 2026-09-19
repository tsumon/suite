using System.IO;
using System.Windows;
using System.Windows.Media.Imaging;

namespace Suite.Capture;

/// <summary>
/// Put an image on the Windows clipboard using materialized standard formats.
///
/// Desktop apps commonly consume CF_BITMAP/CF_DIB, while browsers and chat
/// inputs commonly look for image/png. Keep all three representations, but
/// store PNG in a rewinded, materialized MemoryStream so clipboard consumers
/// receive the raw PNG bytes rather than a serialized .NET byte array.
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

        DataObject data = CreateDataObject(source);
        Clipboard.SetDataObject(data, true);
    }

    internal static DataObject CreateDataObjectForTest(BitmapSource source) => CreateDataObject(source);

    private static DataObject CreateDataObject(BitmapSource source)
    {
        var data = new DataObject();
        data.SetImage(source);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(source));
        using var encoded = new MemoryStream();
        encoder.Save(encoded);
        byte[] png = encoded.ToArray();
        data.SetData("PNG", new MemoryStream(png, writable: false), false);
        data.SetData("image/png", new MemoryStream(png, writable: false), false);
        return data;
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
