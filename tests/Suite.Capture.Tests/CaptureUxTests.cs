using Suite.Capture;

namespace Suite.Capture.Tests;

public sealed class CaptureUxTests
{
    [Fact]
    public void Auto_pin_on_skips_toolbar_without_shift()
    {
        Assert.False(CaptureUx.OpenAnnotationAfterRegion(pinAfterCapture: true, annotateModifier: false));
    }

    [Fact]
    public void Shift_opens_toolbar_even_when_auto_pin_is_on()
    {
        Assert.True(CaptureUx.OpenAnnotationAfterRegion(pinAfterCapture: true, annotateModifier: true));
    }

    [Fact]
    public void Auto_pin_off_opens_toolbar()
    {
        Assert.True(CaptureUx.OpenAnnotationAfterRegion(pinAfterCapture: false, annotateModifier: false));
    }

    [Fact]
    public void Overlay_hint_is_the_default_path_sentence()
    {
        Assert.Equal("悬停点窗，拖动手选。松手出工具栏。Esc 取消。", CaptureUx.OverlayHint);
        Assert.Contains("工具栏", CaptureUx.OverlayHint, StringComparison.Ordinal);
        Assert.DoesNotContain("松手钉图", CaptureUx.OverlayHint, StringComparison.Ordinal);
        Assert.DoesNotContain("Shift", CaptureUx.OverlayHint, StringComparison.Ordinal);
        Assert.DoesNotContain("标注", CaptureUx.OverlayHint, StringComparison.Ordinal);
    }

    [Fact]
    public void Size_chip_uses_spaces_times_and_px()
    {
        Assert.Equal("235 × 172 px", CaptureUx.SizeChip(235, 172));
        Assert.Equal("326 × 206 px", CaptureUx.SizeChip(326, 206));
    }

    [Fact]
    public void Four_pixel_move_is_a_drag_three_is_a_click()
    {
        Assert.False(CaptureUx.IsDrag(0, 0, 3, 0));
        Assert.True(CaptureUx.IsDrag(10, 10, 14, 10));
        Assert.True(CaptureUx.IsDrag(0, 0, 3, 3));
    }

    [Fact]
    public void Click_commits_hover_window()
    {
        Assert.Equal(
            CaptureUx.ReleaseAction.CommitHover,
            CaptureUx.DecideRelease(isDragging: false, dragWidth: 1, dragHeight: 1, hasHover: true));
    }

    [Fact]
    public void Click_without_hover_stays_on_overlay()
    {
        Assert.Equal(
            CaptureUx.ReleaseAction.Stay,
            CaptureUx.DecideRelease(isDragging: false, dragWidth: 0, dragHeight: 0, hasHover: false));
    }

    [Fact]
    public void Drag_at_least_4x4_commits_the_hand_rect()
    {
        Assert.Equal(
            CaptureUx.ReleaseAction.CommitDrag,
            CaptureUx.DecideRelease(isDragging: true, dragWidth: 4, dragHeight: 4, hasHover: true));
    }

    [Fact]
    public void Tiny_drag_falls_back_to_hover_window()
    {
        Assert.Equal(
            CaptureUx.ReleaseAction.CommitHover,
            CaptureUx.DecideRelease(isDragging: true, dragWidth: 2, dragHeight: 2, hasHover: true));
    }

    [Fact]
    public void Tiny_drag_without_hover_stays_on_overlay()
    {
        Assert.Equal(
            CaptureUx.ReleaseAction.Stay,
            CaptureUx.DecideRelease(isDragging: true, dragWidth: 3, dragHeight: 3, hasHover: false));
    }

    [Fact]
    public void Annotate_arm_is_400ms()
    {
        Assert.Equal(400, CaptureUx.AnnotateArmMs);
    }

    [Fact]
    public void Window_snap_click_is_commit_hover_not_drag()
    {
        // WeChat-style window snap: hover + click commits hover rect.
        Assert.False(CaptureUx.IsDrag(100, 100, 102, 101));
        Assert.Equal(
            CaptureUx.ReleaseAction.CommitHover,
            CaptureUx.DecideRelease(isDragging: false, dragWidth: 0, dragHeight: 0, hasHover: true));
    }

    [Fact]
    public void MoveRect_keeps_size_and_clamps()
    {
        var origin = new PixelRect(100, 50, 40, 30);
        var clamp = new PixelRect(0, 0, 200, 150);
        PixelRect moved = CaptureUx.MoveRect(origin, 10, -20, clamp);
        Assert.Equal(110, moved.X);
        Assert.Equal(30, moved.Y);
        Assert.Equal(40, moved.Width);
        Assert.Equal(30, moved.Height);

        PixelRect hitEdge = CaptureUx.MoveRect(origin, 1000, 1000, clamp);
        Assert.Equal(160, hitEdge.X); // 200 - 40
        Assert.Equal(120, hitEdge.Y); // 150 - 30
        Assert.Equal(40, hitEdge.Width);
        Assert.Equal(30, hitEdge.Height);
    }

    [Fact]
    public void MoveRect_clamps_negative_delta_to_bounds_origin()
    {
        var origin = new PixelRect(10, 10, 20, 20);
        var clamp = new PixelRect(0, 0, 100, 100);
        PixelRect moved = CaptureUx.MoveRect(origin, -50, -50, clamp);
        Assert.Equal(0, moved.X);
        Assert.Equal(0, moved.Y);
        Assert.Equal(20, moved.Width);
        Assert.Equal(20, moved.Height);
    }

    [Fact]
    public void UnionMonitorBounds_merges_rects()
    {
        PixelRect union = CaptureUx.UnionMonitorBounds(
            [new PixelRect(0, 0, 100, 80), new PixelRect(100, -20, 50, 100)],
            fallback: new PixelRect(0, 0, 1, 1));
        Assert.Equal(0, union.X);
        Assert.Equal(-20, union.Y);
        Assert.Equal(150, union.Width);
        Assert.Equal(100, union.Height);
    }

    [Fact]
    public void Largest_overlap_wins_not_enumeration_order()
    {
        var monitors = new[]
        {
            new PixelRect(0, 0, 1920, 1080),
            new PixelRect(1920, 0, 1920, 1080),
        };
        // Mostly on secondary
        var selection = new PixelRect(1800, 100, 400, 400);
        Assert.Equal(1, CaptureUx.IndexOfLargestOverlap(monitors, selection));
        // Mostly on primary
        Assert.Equal(0, CaptureUx.IndexOfLargestOverlap(monitors, new PixelRect(100, 100, 400, 400)));
        Assert.Equal(-1, CaptureUx.IndexOfLargestOverlap(monitors, new PixelRect(-5000, -5000, 10, 10)));
    }

    [Fact]
    public void HitTest_interior_is_none()
    {
        var sel = new PixelRect(100, 200, 300, 180);
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 250, 290, gripPx: 10));
    }

    [Fact]
    public void HitTest_corners_win_over_edges()
    {
        var sel = new PixelRect(100, 200, 300, 180);
        const int grip = 10;
        Assert.Equal(CaptureUx.ResizeHandle.NW, CaptureUx.HitTestResizeHandle(sel, 100, 200, grip));
        Assert.Equal(CaptureUx.ResizeHandle.NE, CaptureUx.HitTestResizeHandle(sel, 399, 200, grip));
        Assert.Equal(CaptureUx.ResizeHandle.SW, CaptureUx.HitTestResizeHandle(sel, 100, 379, grip));
        Assert.Equal(CaptureUx.ResizeHandle.SE, CaptureUx.HitTestResizeHandle(sel, 399, 379, grip));
    }

    [Fact]
    public void HitTest_edges_away_from_corners()
    {
        var sel = new PixelRect(100, 200, 300, 180);
        const int grip = 10;
        Assert.Equal(CaptureUx.ResizeHandle.N, CaptureUx.HitTestResizeHandle(sel, 250, 205, grip));
        Assert.Equal(CaptureUx.ResizeHandle.S, CaptureUx.HitTestResizeHandle(sel, 250, 375, grip));
        Assert.Equal(CaptureUx.ResizeHandle.W, CaptureUx.HitTestResizeHandle(sel, 105, 290, grip));
        Assert.Equal(CaptureUx.ResizeHandle.E, CaptureUx.HitTestResizeHandle(sel, 395, 290, grip));
    }

    [Fact]
    public void HitTest_outside_selection_is_none()
    {
        var sel = new PixelRect(100, 200, 300, 180);
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 99, 200, gripPx: 10));
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 400, 290, gripPx: 10));
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 250, 380, gripPx: 10));
    }

    [Fact]
    public void HitTest_empty_or_bad_grip_is_none()
    {
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(default, 0, 0, gripPx: 10));
        var sel = new PixelRect(0, 0, 50, 50);
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 1, 1, gripPx: 0));
    }

    [Fact]
    public void HitTest_beyond_grip_interior_is_none()
    {
        var sel = new PixelRect(0, 0, 100, 100);
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 50, 50, gripPx: 10));
        // Just outside grip band on north edge center
        Assert.Equal(
            CaptureUx.ResizeHandle.None,
            CaptureUx.HitTestResizeHandle(sel, 50, 10, gripPx: 10));
        Assert.Equal(
            CaptureUx.ResizeHandle.N,
            CaptureUx.HitTestResizeHandle(sel, 50, 9, gripPx: 10));
    }

    [Fact]
    public void Resize_grip_default_is_ten()
    {
        Assert.Equal(10, CaptureUx.ResizeGripPx);
    }
}
