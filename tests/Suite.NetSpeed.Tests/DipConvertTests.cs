using Suite.Platform;

namespace Suite.NetSpeed.Tests;

public sealed class DipConvertTests
{
    [Fact]
    public void PixelsToDip_is_identity_at_96()
    {
        Assert.Equal(200, DipConvert.PixelsToDip(200, 96), 6);
    }

    [Fact]
    public void PixelsToDip_shrinks_on_144_dpi_so_pin_is_1_to_1()
    {
        // 200 physical pixels on 150% (144 DPI) must be 200 * 96 / 144 DIP.
        Assert.Equal(200.0 * 96.0 / 144.0, DipConvert.PixelsToDip(200, 144), 6);
    }

    [Fact]
    public void PixelsToDip_shrinks_on_192_dpi()
    {
        Assert.Equal(100, DipConvert.PixelsToDip(200, 192), 6);
    }

    [Fact]
    public void PixelsToDip_treats_zero_dpi_as_96()
    {
        Assert.Equal(80, DipConvert.PixelsToDip(80, 0), 6);
    }

    [Fact]
    public void Size_uses_each_axis_dpi()
    {
        var (width, height) = DipConvert.Size(192, 96, 144, 96);
        Assert.Equal(128, width, 6);
        Assert.Equal(96, height, 6);
    }

    [Fact]
    public void Origin_maps_virtual_pixels_to_dip()
    {
        var (left, top) = DipConvert.Origin(144, 288, 144, 144);
        Assert.Equal(96, left, 6);
        Assert.Equal(192, top, 6);
    }
}
