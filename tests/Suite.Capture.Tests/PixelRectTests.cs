using Suite.Capture;

namespace Suite.Capture.Tests;

public sealed class PixelRectTests
{
    [Fact]
    public void FromCorners_normalizes_inverted_drag()
    {
        PixelRect rect = PixelRect.FromCorners(10, 8, 2, 1);
        Assert.Equal(2, rect.X);
        Assert.Equal(1, rect.Y);
        Assert.Equal(8, rect.Width);
        Assert.Equal(7, rect.Height);
        Assert.Equal(10, rect.Right);
        Assert.Equal(8, rect.Bottom);
    }

    [Fact]
    public void Intersect_empty_when_no_overlap()
    {
        var a = new PixelRect(0, 0, 10, 10);
        var b = new PixelRect(20, 20, 5, 5);
        Assert.True(a.Intersect(b).IsEmpty);
    }

    [Fact]
    public void Intersect_clips_to_overlap()
    {
        var a = new PixelRect(0, 0, 10, 10);
        var b = new PixelRect(8, 8, 10, 10);
        PixelRect i = a.Intersect(b);
        Assert.Equal(new PixelRect(8, 8, 2, 2), i);
    }

    [Fact]
    public void ClampTo_stays_inside_buffer()
    {
        PixelRect clipped = new PixelRect(-4, -4, 10, 10).ClampTo(8, 8);
        Assert.Equal(new PixelRect(0, 0, 6, 6), clipped);
    }
}
