using Suite.Capture;

namespace Suite.Capture.Tests;

public sealed class AnnotateToolbarTests
{
    [Fact]
    public void Order_matches_snipaste_live_toolbar()
    {
        Assert.Equal(
            new[]
            {
                AnnotateTool.Shape,
                AnnotateTool.Curve,
                AnnotateTool.Pencil,
                AnnotateTool.Marker,
                AnnotateTool.Mosaic,
                AnnotateTool.Text,
                AnnotateTool.Ocr,
                AnnotateTool.Pick,
                AnnotateTool.Eraser,
                AnnotateTool.Undo,
                AnnotateTool.Redo,
                AnnotateTool.Close,
                AnnotateTool.Pin,
                AnnotateTool.Save,
                AnnotateTool.Copy,
            },
            AnnotateToolbar.Order);
        Assert.Equal(15, AnnotateToolbar.Order.Length);
    }

    [Fact]
    public void Tooltips_are_short_chinese_with_no_button_labels()
    {
        Assert.Equal("矩形", AnnotateToolbar.Tooltip(AnnotateTool.Shape));
        Assert.Equal("曲线", AnnotateToolbar.Tooltip(AnnotateTool.Curve));
        Assert.Equal("铅笔", AnnotateToolbar.Tooltip(AnnotateTool.Pencil));
        Assert.Equal("马克笔", AnnotateToolbar.Tooltip(AnnotateTool.Marker));
        Assert.Equal("马赛克", AnnotateToolbar.Tooltip(AnnotateTool.Mosaic));
        Assert.Equal("文字", AnnotateToolbar.Tooltip(AnnotateTool.Text));
        Assert.Equal("识字", AnnotateToolbar.Tooltip(AnnotateTool.Ocr));
        Assert.Equal("取色", AnnotateToolbar.Tooltip(AnnotateTool.Pick));
        Assert.Equal("橡皮", AnnotateToolbar.Tooltip(AnnotateTool.Eraser));
        Assert.Equal("撤销", AnnotateToolbar.Tooltip(AnnotateTool.Undo));
        Assert.Equal("重做", AnnotateToolbar.Tooltip(AnnotateTool.Redo));
        Assert.Equal("关闭", AnnotateToolbar.Tooltip(AnnotateTool.Close));
        Assert.Equal("钉图", AnnotateToolbar.Tooltip(AnnotateTool.Pin));
        Assert.Equal("保存", AnnotateToolbar.Tooltip(AnnotateTool.Save));
        Assert.Equal("复制", AnnotateToolbar.Tooltip(AnnotateTool.Copy));
    }

    [Fact]
    public void Draw_tools_take_the_red_dot_submit_tools_do_not()
    {
        Assert.True(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Curve));
        Assert.True(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Pencil));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Undo));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Close));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Pin));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Save));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Copy));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Ocr));
        Assert.False(AnnotateToolbar.ShowsSelectedDot(AnnotateTool.Pick));
    }

    [Fact]
    public void Place_prefers_below_and_right_aligns_when_bar_is_wider()
    {
        // 07: selection 297,85 326x206; bar ~530x40; work 0,0,1920,1080
        AnnotateToolbar.Placement place = AnnotateToolbar.Place(
            selLeft: 297, selTop: 85, selWidth: 326, selHeight: 206,
            barWidth: 530, barHeight: 40,
            workLeft: 0, workTop: 0, workRight: 1920, workBottom: 1080);
        Assert.False(place.Above);
        Assert.Equal(85 + 206 + 8, place.Top);
        Assert.Equal(297 + 326 - 530, place.Left);
    }

    [Fact]
    public void Place_flips_above_when_below_would_leave_the_work_area()
    {
        AnnotateToolbar.Placement place = AnnotateToolbar.Place(
            selLeft: 100, selTop: 1000, selWidth: 200, selHeight: 50,
            barWidth: 500, barHeight: 40,
            workLeft: 0, workTop: 0, workRight: 1920, workBottom: 1080);
        Assert.True(place.Above);
        Assert.Equal(1000 - 8 - 40, place.Top);
    }

    [Fact]
    public void Place_left_aligns_when_bar_is_narrower_than_selection()
    {
        AnnotateToolbar.Placement place = AnnotateToolbar.Place(
            selLeft: 400, selTop: 100, selWidth: 800, selHeight: 200,
            barWidth: 500, barHeight: 40,
            workLeft: 0, workTop: 0, workRight: 1920, workBottom: 1080);
        Assert.Equal(400, place.Left);
        Assert.False(place.Above);
    }

    [Fact]
    public void Place_left_aligns_to_selection_when_right_align_would_leave_the_work_area()
    {
        AnnotateToolbar.Placement place = AnnotateToolbar.Place(
            selLeft: 10, selTop: 100, selWidth: 80, selHeight: 60,
            barWidth: 500, barHeight: 40,
            workLeft: 0, workTop: 0, workRight: 1920, workBottom: 1080);
        Assert.Equal(10, place.Left);
    }

    [Fact]
    public void Place_clamps_into_work_when_bar_still_cannot_fit()
    {
        AnnotateToolbar.Placement place = AnnotateToolbar.Place(
            selLeft: 10, selTop: 100, selWidth: 80, selHeight: 60,
            barWidth: 500, barHeight: 40,
            workLeft: 0, workTop: 0, workRight: 400, workBottom: 1080);
        Assert.Equal(0, place.Left);
    }

    [Fact]
    public void ShowsPropertyBar_for_drawing_tools_only()
    {
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Curve));
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Shape));
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Pencil));
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Marker));
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Mosaic));
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Text));
        Assert.True(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Eraser));
        Assert.False(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Ocr));
        Assert.False(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Pick));
        Assert.False(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Undo));
        Assert.False(AnnotateToolbar.ShowsPropertyBar(AnnotateTool.Copy));
    }

    [Fact]
    public void PlaceStack_puts_sub_under_main_when_below_selection()
    {
        AnnotateToolbar.StackPlacement place = AnnotateToolbar.PlaceStack(
            selLeft: 100, selTop: 100, selWidth: 400, selHeight: 200,
            barWidth: 500, mainHeight: 40, subHeight: 36, showSub: true,
            workLeft: 0, workTop: 0, workRight: 1920, workBottom: 1080);
        Assert.False(place.Above);
        Assert.True(place.SubVisible);
        Assert.Equal(100 + 200 + 8, place.MainTop);
        Assert.Equal(place.MainTop + 40 + AnnotateToolbar.SubBarGap, place.SubTop);
    }

    [Fact]
    public void PlaceStack_puts_sub_above_main_when_stack_flips_above()
    {
        AnnotateToolbar.StackPlacement place = AnnotateToolbar.PlaceStack(
            selLeft: 100, selTop: 1000, selWidth: 200, selHeight: 50,
            barWidth: 500, mainHeight: 40, subHeight: 36, showSub: true,
            workLeft: 0, workTop: 0, workRight: 1920, workBottom: 1080);
        Assert.True(place.Above);
        Assert.True(place.SubVisible);
        Assert.True(place.SubTop < place.MainTop);
        Assert.Equal(place.SubTop + 36 + AnnotateToolbar.SubBarGap, place.MainTop);
    }

    [Fact]
    public void Palette_has_at_least_six_swatches_including_red()
    {
        Assert.True(AnnotateToolbar.Palette.Length >= 6);
        Assert.Contains(AnnotateToolbar.Palette, c => c.R == 0xDC && c.G == 0x28 && c.B == 0x28);
    }

}
