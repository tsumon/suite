namespace Suite.Capture;

/// <summary>
/// Snipaste-style annotate toolbar contract from design/SNIPASTE-TOOLBAR-SPEC.md.
/// </summary>
public enum AnnotateTool
{
    Shape,
    Curve,
    Pencil,
    Marker,
    Mosaic,
    Text,
    Ocr,
    Pick,
    Eraser,
    Undo,
    Redo,
    Close,
    Pin,
    Save,
    Copy,
}

public static class AnnotateToolbar
{
    public const double Height = 40;
    public const double Cell = 36;
    public const double Gap = 8;
    public const double SeparatorWidth = 8;
    public const double IconSize = 16;
    public const double SelectedDot = 5;
    public const int StrokePx = 3;
    public const int AnchorDiameter = 8;

    public static readonly AnnotateTool[] Order =
    [
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
    ];

    public readonly record struct Placement(double Left, double Top, bool Above);

    public static string Tooltip(AnnotateTool tool) => tool switch
    {
        AnnotateTool.Shape => "矩形",
        AnnotateTool.Curve => "曲线",
        AnnotateTool.Pencil => "铅笔",
        AnnotateTool.Marker => "马克笔",
        AnnotateTool.Mosaic => "马赛克",
        AnnotateTool.Text => "文字",
        AnnotateTool.Eraser => "橡皮",
        AnnotateTool.Undo => "撤销",
        AnnotateTool.Redo => "重做",
        AnnotateTool.Close => "关闭",
        AnnotateTool.Pin => "钉图",
        AnnotateTool.Save => "保存",
        AnnotateTool.Ocr => "识字",
        AnnotateTool.Pick => "取色",
        AnnotateTool.Copy => "复制",
        _ => "",
    };

    public static bool ShowsSelectedDot(AnnotateTool tool) => tool is
        AnnotateTool.Shape or
        AnnotateTool.Curve or
        AnnotateTool.Pencil or
        AnnotateTool.Marker or
        AnnotateTool.Mosaic or
        AnnotateTool.Text or
        AnnotateTool.Eraser;

    public static bool IsHistory(AnnotateTool tool) => tool is AnnotateTool.Undo or AnnotateTool.Redo;

    public static Placement Place(
        double selLeft,
        double selTop,
        double selWidth,
        double selHeight,
        double barWidth,
        double barHeight,
        double workLeft,
        double workTop,
        double workRight,
        double workBottom,
        double gap = Gap)
    {
        bool above;
        double top;
        if (selTop + selHeight + gap + barHeight <= workBottom)
        {
            top = selTop + selHeight + gap;
            above = false;
        }
        else if (selTop - gap - barHeight >= workTop)
        {
            top = selTop - gap - barHeight;
            above = true;
        }
        else
        {
            double maxTop = Math.Max(workTop, workBottom - barHeight);
            top = Math.Clamp(selTop + selHeight + gap, workTop, maxTop);
            above = false;
        }

        double left;
        if (barWidth > selWidth)
        {
            left = selLeft + selWidth - barWidth;
            if (left < workLeft)
            {
                left = selLeft;
            }
        }
        else
        {
            left = selLeft;
        }

        if (left + barWidth > workRight)
        {
            left = workRight - barWidth;
        }

        if (left < workLeft)
        {
            left = workLeft;
        }

        return new Placement(left, top, above);
    }
}
