using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;

namespace Suite.Capture;

internal static class AnnotateIcons
{
    private static readonly Brush Ink = Freeze(Color.FromRgb(0x32, 0x32, 0x32));

    public static FrameworkElement Glyph(AnnotateTool tool)
    {
        return tool switch
        {
            AnnotateTool.Shape => Shape(),
            AnnotateTool.Curve => Stroke("M2,12 L6,5 L10,11 L14,4"),
            AnnotateTool.Pencil => Stroke("M3.5,13.5 L5.5,13.5 L13,6 L10,3 L2.5,10.5 Z M10.2,3.2 L12.8,5.8"),
            AnnotateTool.Marker => Stroke("M4,13 L6.5,4 H9.5 L12,13 M5.2,10 H10.8"),
            AnnotateTool.Mosaic => Mosaic(),
            AnnotateTool.Text => Stroke("M3,3.5 H13 M8,3.5 V13.5 M4.5,13.5 H11.5"),
            AnnotateTool.Eraser => Stroke("M3,10 L7.5,4.5 L13,10 L10,13.5 H6 Z"),
            AnnotateTool.Undo => Stroke("M4,8 H11 A3,3 0 1 1 11,12 H8 M4,8 L6.5,5.5 M4,8 L6.5,10.5"),
            AnnotateTool.Redo => Stroke("M12,8 H5 A3,3 0 1 0 5,12 H8 M12,8 L9.5,5.5 M12,8 L9.5,10.5"),
            AnnotateTool.Close => Stroke("M4,4 L12,12 M12,4 L4,12"),
            AnnotateTool.Pin => Pin(),
            AnnotateTool.Save => Stroke("M3.5,3.5 H11 L13.5,6 V13.5 H3.5 Z M6,3.5 V7 H11 V3.5 M5.5,10.5 H11.5"),
            AnnotateTool.Ocr => Ocr(),
            AnnotateTool.Pick => Pick(),
            AnnotateTool.Copy => Copy(),
            _ => Stroke("M3,3 H13 V13 H3 Z"),
        };
    }

    private static FrameworkElement Shape()
    {
        var grid = new Grid { Width = 16, Height = 16 };
        grid.Children.Add(Stroke("M2,4 H12 V13 H2 Z"));
        var tri = new Path
        {
            Data = Geometry.Parse("M11.5,11 L15,11 L13.25,14.5 Z"),
            Fill = Ink,
            Stroke = Ink,
            StrokeThickness = 0.6,
            Stretch = Stretch.None,
            Width = 16,
            Height = 16,
        };
        grid.Children.Add(tri);
        return grid;
    }

    private static FrameworkElement Mosaic()
    {
        var grid = new Grid { Width = 16, Height = 16 };
        grid.Children.Add(Box(2.5, 2.5));
        grid.Children.Add(Box(8.5, 2.5));
        grid.Children.Add(Box(2.5, 8.5));
        grid.Children.Add(Box(8.5, 8.5));
        return grid;
    }

    private static FrameworkElement Box(double x, double y)
    {
        return new System.Windows.Shapes.Rectangle
        {
            Width = 5,
            Height = 5,
            Stroke = Ink,
            StrokeThickness = 1.2,
            Fill = Brushes.Transparent,
            Margin = new Thickness(x, y, 0, 0),
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Top,
        };
    }

    private static FrameworkElement Pin()
    {
        var grid = new Grid { Width = 16, Height = 16 };
        grid.Children.Add(Stroke("M8,8.5 V14"));
        var head = new Path
        {
            Data = Geometry.Parse("M5.5,2.5 H10.5 L11.5,7.2 H4.5 Z"),
            Stroke = Ink,
            StrokeThickness = 1.4,
            StrokeLineJoin = PenLineJoin.Round,
            Fill = Brushes.Transparent,
            Stretch = Stretch.None,
            Width = 16,
            Height = 16,
        };
        grid.Children.Add(head);
        return grid;
    }

    private static FrameworkElement Pick()
    {
        // Eyedropper-ish: stem + tip
        var grid = new Grid { Width = 16, Height = 16 };
        grid.Children.Add(Stroke("M4.5,11.5 L10.5,5.5 M9.2,4.2 L11.8,6.8 M10.5,5.5 L13,3"));
        grid.Children.Add(Stroke("M3.5,12.5 L6,14.5 L5,11 Z"));
        return grid;
    }

    private static FrameworkElement Ocr()
    {
        // 识字：T + 底横线（OCR-SCROLL-SPEC §1.1），16px 描边
        var grid = new Grid { Width = 16, Height = 16 };
        grid.Children.Add(Stroke("M3,3.5 H13 M8,3.5 V11.5"));
        grid.Children.Add(Stroke("M3.5,14 H12.5"));
        return grid;
    }

    private static FrameworkElement Copy()
    {
        var grid = new Grid { Width = 16, Height = 16 };
        grid.Children.Add(Stroke("M5.5,5.5 H13.5 V13.5 H5.5 Z"));
        grid.Children.Add(Stroke("M2.5,2.5 H10.5 V5"));
        return grid;
    }

    private static Path Stroke(string data) => new()
    {
        Data = Geometry.Parse(data),
        Stroke = Ink,
        StrokeThickness = 1.5,
        StrokeStartLineCap = PenLineCap.Round,
        StrokeEndLineCap = PenLineCap.Round,
        StrokeLineJoin = PenLineJoin.Round,
        Fill = Brushes.Transparent,
        Stretch = Stretch.None,
        Width = 16,
        Height = 16,
        SnapsToDevicePixels = true,
    };

    private static SolidColorBrush Freeze(Color color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }
}
