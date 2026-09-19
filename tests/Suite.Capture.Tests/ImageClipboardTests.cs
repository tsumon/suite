using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Suite.Capture;

namespace Suite.Capture.Tests;

public sealed class ImageClipboardTests
{
    [Fact]
    public void Standard_and_materialized_png_formats_are_available()
    {
        var source = BitmapSource.Create(
            1,
            1,
            96,
            96,
            PixelFormats.Bgra32,
            null,
            new byte[] { 0, 0, 0, 255 },
            4);

        DataObject data = ImageClipboard.CreateDataObjectForTest(source);
        string[] formats = data.GetFormats();

        Assert.Contains(DataFormats.Bitmap, formats);
        Assert.Contains("PNG", formats);
        Assert.Contains("image/png", formats);
        Assert.IsAssignableFrom<Stream>(data.GetData("PNG"));
        Assert.IsAssignableFrom<Stream>(data.GetData("image/png"));
    }
}
