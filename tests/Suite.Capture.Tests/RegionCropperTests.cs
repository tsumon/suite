using Suite.Capture;

namespace Suite.Capture.Tests;

public sealed class RegionCropperTests
{
    [Fact]
    public void Crop_stitches_two_monitors()
    {
        PixelBuffer left = PixelBuffer.Allocate(10, 10);
        PixelDraw.FillColor(left, new PixelRect(0, 0, 10, 10), 10, 0, 0, 255);
        PixelBuffer right = PixelBuffer.Allocate(10, 10);
        PixelDraw.FillColor(right, new PixelRect(0, 0, 10, 10), 0, 0, 40, 255);

        var frames = new[]
        {
            new MonitorCapture
            {
                Monitor = new MonitorInfo { Bounds = new PixelRect(0, 0, 10, 10), DpiX = 96, DpiY = 96 },
                Buffer = left,
                Method = CaptureMethod.GraphicsCapture,
            },
            new MonitorCapture
            {
                Monitor = new MonitorInfo { Bounds = new PixelRect(10, 0, 10, 10), DpiX = 96, DpiY = 96 },
                Buffer = right,
                Method = CaptureMethod.GdiBitBlt,
            },
        };

        PixelBuffer crop = RegionCropper.Crop(frames, new PixelRect(8, 0, 4, 2));
        Assert.Equal(4, crop.Width);
        Assert.Equal(2, crop.Height);
        Assert.Equal(10, crop.Bgra[0]);
        Assert.Equal(40, crop.Bgra[(2 * 4) + 2]);
        Assert.Equal(CaptureMethod.GdiBitBlt, RegionCropper.DominantMethod(frames, new PixelRect(9, 0, 4, 2)));
    }
}
