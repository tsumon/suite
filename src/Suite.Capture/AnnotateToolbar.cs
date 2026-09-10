namespace Suite.Capture;

/// <summary>Curve / line style for AnnotateTool.Curve (secondary bar + right-click fallback).</summary>
public enum CurveStyle
{
    Solid,
    Dashed,
    Arrow,
}

/// <summary>
/// Snipaste-style annotate toolbar contract from design/SNIPASTE-TOOLBAR-SPEC.md.
/// Secondary (腾讯/WeChat) property strip sits under the main bar for drawing tools.
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
    public const double SubBarHeight = 36;
    public const double SubBarGap = 4;
    public const double Cell = 36;
    public const double Gap = 8;
    public const double SeparatorWidth = 8;
    public const double IconSize = 16;
    public const double SelectedDot = 5;
    public const int StrokePx = 3;
    public const int AnchorDiameter = 12;

    /// <summary>RGB swatches matching Tencent-style annotate palette (cyan…red).</summary>
    public static readonly (byte R, byte G, byte B)[] Palette =
    [
        (0x00, 0xD4, 0xFF),
        (0x00, 0xE6, 0x76),
        (0xFF, 0xEB, 0x3B),
        (0x9E, 0x9E, 0x9E),
        (0xFF, 0xFF, 0xFF),
        (0xDC, 0x28, 0x28),
    ];

    public static readonly int[] ThicknessChips = [2, 4, 6];
    public static readonly int[] MarkerThicknessChips = [10, 14, 20];
    public static readonly int[] MosaicRadiusChips = [10, 16, 24];
    public static readonly int[] EraserThicknessChips = [12, 18, 28];
    public static readonly int[] TextSizeChips = [14, 18, 24];

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

    /// <summary>Main + optional secondary property bar stack placement.</summary>
    public readonly record struct StackPlacement(
        double Left,
        double MainTop,
        double SubTop,
        bool Above,
        bool SubVisible);

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

    /// <summary>Drawing tools that show the Tencent-style secondary property strip.</summary>
    public static bool ShowsPropertyBar(AnnotateTool tool) => tool is
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

    /// <summary>
    /// Place main toolbar + optional secondary strip as one stack.
    /// Below selection: [main] then [sub]. Above selection: [sub] above [main] (main closest to sel).
    /// </summary>
    public static StackPlacement PlaceStack(
        double selLeft,
        double selTop,
        double selWidth,
        double selHeight,
        double barWidth,
        double mainHeight,
        double subHeight,
        bool showSub,
        double workLeft,
        double workTop,
        double workRight,
        double workBottom,
        double gap = Gap)
    {
        double stackH = showSub ? mainHeight + SubBarGap + subHeight : mainHeight;
        Placement place = Place(
            selLeft, selTop, selWidth, selHeight,
            barWidth, stackH,
            workLeft, workTop, workRight, workBottom,
            gap);

        if (!showSub)
        {
            return new StackPlacement(place.Left, place.Top, place.Top, place.Above, false);
        }

        if (!place.Above)
        {
            return new StackPlacement(
                place.Left,
                place.Top,
                place.Top + mainHeight + SubBarGap,
                Above: false,
                SubVisible: true);
        }

        // Stack above selection: sub on top, main under it (closer to selection).
        return new StackPlacement(
            place.Left,
            place.Top + subHeight + SubBarGap,
            place.Top,
            Above: true,
            SubVisible: true);
    }
}
