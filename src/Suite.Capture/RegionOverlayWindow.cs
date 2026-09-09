using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;
using System.Windows.Threading;

namespace Suite.Capture;

internal sealed class RegionOverlayWindow : Window
{
    private static readonly SolidColorBrush DimBrush = FreezeBrush(Color.FromArgb(0x6E, 0, 0, 0));
    private static readonly SolidColorBrush AccentBrush = FreezeBrush(Color.FromRgb(0x20, 0x80, 0xF0));
    private static readonly SolidColorBrush HoverBrush = FreezeBrush(Color.FromRgb(0xDC, 0x28, 0x28));
    private static readonly SolidColorBrush ChipBrush = FreezeBrush(Color.FromArgb(0xCC, 0, 0, 0));

    private readonly MonitorCapture _frame;
    private readonly RegionSelectSession _session;
    private readonly Canvas _mask = new();
    private readonly Rectangle _top = new();
    private readonly Rectangle _bottom = new();
    private readonly Rectangle _left = new();
    private readonly Rectangle _right = new();
    private readonly Rectangle _border = new();
    private readonly Border _sizeChip = new();
    private readonly TextBlock _sizeText = new();
    private readonly double _dipX;
    private readonly double _dipY;
    private readonly bool _showMagnifier;
    private readonly Border _magnifier = new();
    private readonly Image _magImage = new();
    private readonly TextBlock _magLabel = new();
    private readonly Image _backdrop = new();
    private string? _magFailHint;

    public RegionOverlayWindow(MonitorCapture frame, RegionSelectSession session, bool showMagnifier = true)
    {
        _frame = frame;
        _session = session;
        _showMagnifier = showMagnifier;
        _dipX = 96.0 / Math.Max(1, (int)frame.Monitor.DpiX);
        _dipY = 96.0 / Math.Max(1, (int)frame.Monitor.DpiY);

        WindowStyle = WindowStyle.None;
        ResizeMode = ResizeMode.NoResize;
        ShowInTaskbar = false;
        Topmost = true;
        Cursor = Cursors.Cross;
        Background = Brushes.Black;
        Focusable = true;
        Title = "Suite 截图";

        Left = frame.Monitor.Bounds.X * _dipX;
        Top = frame.Monitor.Bounds.Y * _dipY;
        Width = frame.Monitor.Bounds.Width * _dipX;
        Height = frame.Monitor.Bounds.Height * _dipY;

        _backdrop.Source = frame.Buffer.ToBitmapSource();
        _backdrop.Stretch = Stretch.Fill;
        _backdrop.SnapsToDevicePixels = true;
        foreach (Rectangle rect in new[] { _top, _bottom, _left, _right })
        {
            rect.Fill = DimBrush;
            rect.IsHitTestVisible = false;
            _mask.Children.Add(rect);
        }

        _border.Stroke = AccentBrush;
        _border.StrokeThickness = AnnotateToolbar.StrokePx;
        _border.Fill = Brushes.Transparent;
        _border.IsHitTestVisible = false;
        _mask.Children.Add(_border);

        _sizeText.Foreground = Brushes.White;
        _sizeText.FontFamily = new FontFamily("Segoe UI, Consolas, Microsoft YaHei UI");
        _sizeText.FontSize = 11;
        _sizeChip.Background = ChipBrush;
        _sizeChip.CornerRadius = new CornerRadius(3);
        _sizeChip.Padding = new Thickness(6, 2, 6, 2);
        _sizeChip.Child = _sizeText;
        _sizeChip.IsHitTestVisible = false;
        _sizeChip.Visibility = Visibility.Collapsed;
        _mask.Children.Add(_sizeChip);

        var hint = new TextBlock
        {
            Text = session.OverlayHintText ?? CaptureUx.OverlayHint,
            Foreground = Brushes.White,
            FontFamily = new FontFamily("Segoe UI, Microsoft YaHei UI"),
            FontSize = 13,
            Margin = new Thickness(16),
            IsHitTestVisible = false,
        };
        Canvas.SetLeft(hint, 0);
        Canvas.SetTop(hint, 0);
        _mask.Children.Add(hint);

        if (_showMagnifier)
        {
            BuildMagnifier();
            _mask.Children.Add(_magnifier);
        }

        var grid = new Grid();
        grid.Children.Add(_backdrop);
        grid.Children.Add(_mask);
        Content = grid;

        Loaded += (_, _) =>
        {
            Activate();
            Keyboard.Focus(this);
            Redraw();
        };
        MouseEnter += (_, _) => Keyboard.Focus(this);
        PreviewKeyDown += OnKey;
        PreviewKeyUp += OnKeyUp;
        MouseLeftButtonDown += OnDown;
        MouseMove += OnMove;
        MouseLeftButtonUp += OnUp;
        MouseRightButtonUp += (_, _) =>
        {
            if (!_session.IsAnnotating)
            {
                _session.Cancel();
            }
        };
        _session.Changed += OnSessionChanged;
        Closed += (_, _) =>
        {
            _session.Changed -= OnSessionChanged;
            ReleaseUiImages();
        };
    }

    /// <summary>Drop full-monitor backdrop + magnifier BitmapSources so WorkingSet can fall after select ends.</summary>
    private void ReleaseUiImages()
    {
        _backdrop.Source = null;
        _magImage.Source = null;
    }

    private void OnSessionChanged() => Dispatcher.BeginInvoke(Redraw, DispatcherPriority.Render);

    private void OnKey(object sender, KeyEventArgs e)
    {
        if (_session.IsAnnotating)
        {
            return;
        }

        if (e.Key == Key.Escape)
        {
            _session.Cancel();
            e.Handled = true;
        }
    }

    private void OnKeyUp(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.LeftShift || e.Key == Key.RightShift)
        {
            _session.NotifyShiftUp();
        }
    }

    private void OnDown(object sender, MouseButtonEventArgs e)
    {
        if (_session.IsAnnotating)
        {
            return;
        }

        Keyboard.Focus(this);
        CaptureMouse();
        _session.Press(ToVirtual(e.GetPosition(this)));
        e.Handled = true;
    }

    private void OnMove(object sender, MouseEventArgs e)
    {
        if (_session.IsAnnotating)
        {
            return;
        }

        var virt = ToVirtual(e.GetPosition(this));
        _session.Move(virt);
        UpdateMagnifier(e.GetPosition(this), virt);
        e.Handled = true;
    }

    private void OnUp(object sender, MouseButtonEventArgs e)
    {
        if (_session.IsAnnotating)
        {
            return;
        }

        if (IsMouseCaptured)
        {
            ReleaseMouseCapture();
        }

        bool shiftDown = (Keyboard.Modifiers & ModifierKeys.Shift) == ModifierKeys.Shift;
        bool annotate = _session.WantsAnnotate(shiftDown);
        _session.Release(ToVirtual(e.GetPosition(this)), annotate);
        e.Handled = true;
    }

    private (int X, int Y) ToVirtual(Point dip)
    {
        int x = _frame.Monitor.Bounds.X + (int)Math.Round(dip.X / _dipX);
        int y = _frame.Monitor.Bounds.Y + (int)Math.Round(dip.Y / _dipY);
        return (x, y);
    }

    private void Redraw()
    {
        double w = ActualWidth > 0 ? ActualWidth : Width;
        double h = ActualHeight > 0 ? ActualHeight : Height;

        PixelRect? selection = _session.Highlight;
        if (selection is null)
        {
            IsHitTestVisible = !_session.IsAnnotating;
            SetRect(_top, 0, 0, w, h);
            SetRect(_bottom, 0, 0, 0, 0);
            SetRect(_left, 0, 0, 0, 0);
            SetRect(_right, 0, 0, 0, 0);
            _border.Visibility = Visibility.Collapsed;
            _sizeChip.Visibility = Visibility.Collapsed;
            return;
        }

        PixelRect local = selection.Value.Intersect(_frame.Monitor.Bounds);
        if (local.IsEmpty)
        {
            IsHitTestVisible = !_session.IsAnnotating;
            SetRect(_top, 0, 0, w, h);
            SetRect(_bottom, 0, 0, 0, 0);
            SetRect(_left, 0, 0, 0, 0);
            SetRect(_right, 0, 0, 0, 0);
            _border.Visibility = Visibility.Collapsed;
            _sizeChip.Visibility = Visibility.Collapsed;
            return;
        }

        double x = (local.X - _frame.Monitor.Bounds.X) * _dipX;
        double y = (local.Y - _frame.Monitor.Bounds.Y) * _dipY;
        double rw = local.Width * _dipX;
        double rh = local.Height * _dipY;
        SetRect(_top, 0, 0, w, y);
        SetRect(_bottom, 0, y + rh, w, Math.Max(0, h - y - rh));
        SetRect(_left, 0, y, x, rh);
        SetRect(_right, x + rw, y, Math.Max(0, w - x - rw), rh);

        bool annotate = _session.IsAnnotating;
        // Belt-and-suspenders: while annotate chrome is up, do not steal mouse/focus.
        IsHitTestVisible = !annotate;
        // DESIGN: hover/drag = red 3px; committed chrome = blue.
        bool hoverOrDrag = !annotate && (_session.IsDragging || _session.Committed is null);
        _border.Stroke = hoverOrDrag ? HoverBrush : AccentBrush;
        _border.StrokeDashArray = null; // solid only; Joe rejected dashed frames
        _border.StrokeThickness = hoverOrDrag ? 3 : AnnotateToolbar.StrokePx;
        Canvas.SetLeft(_border, x);
        Canvas.SetTop(_border, y);
        _border.Width = Math.Max(1, rw);
        _border.Height = Math.Max(1, rh);
        _border.Visibility = annotate ? Visibility.Collapsed : Visibility.Visible;

        if (annotate)
        {
            _sizeChip.Visibility = Visibility.Collapsed;
            _magnifier.Visibility = Visibility.Collapsed;
            return;
        }

        PixelRect chipSource = _session.Drag ?? _session.Hover ?? selection.Value;
        _sizeText.Text = CaptureUx.SizeChip(chipSource.Width, chipSource.Height);
        _sizeChip.Visibility = Visibility.Visible;
        _sizeChip.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
        double chipW = Math.Max(8, _sizeChip.DesiredSize.Width);
        double chipH = Math.Max(8, _sizeChip.DesiredSize.Height);
        double chipX = x + ((rw - chipW) / 2);
        double chipY = y - chipH - 2;
        if (chipY < 0)
        {
            chipY = y + 4;
        }

        if (chipX < 0)
        {
            chipX = 0;
        }

        if (chipX + chipW > w)
        {
            chipX = Math.Max(0, w - chipW);
        }

        Canvas.SetLeft(_sizeChip, chipX);
        Canvas.SetTop(_sizeChip, chipY);
    }

    private static void SetRect(Rectangle rect, double x, double y, double w, double h)
    {
        Canvas.SetLeft(rect, x);
        Canvas.SetTop(rect, y);
        rect.Width = Math.Max(0, w);
        rect.Height = Math.Max(0, h);
    }

    private static SolidColorBrush FreezeBrush(Color color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }

    private void BuildMagnifier()
    {
        _magnifier.Width = 120;
        _magnifier.Height = 120;
        _magnifier.CornerRadius = new CornerRadius(2);
        _magnifier.BorderBrush = Brushes.White;
        _magnifier.BorderThickness = new Thickness(1);
        _magnifier.Background = Brushes.Black;
        _magnifier.IsHitTestVisible = false;
        _magnifier.Visibility = Visibility.Collapsed;
        var dock = new DockPanel();
        _magLabel.Foreground = Brushes.White;
        _magLabel.FontSize = 11;
        _magLabel.FontFamily = new FontFamily("Consolas, Segoe UI, Microsoft YaHei UI");
        _magLabel.Padding = new Thickness(4, 2, 4, 2);
        _magLabel.HorizontalAlignment = HorizontalAlignment.Center;
        DockPanel.SetDock(_magLabel, Dock.Bottom);
        _magImage.Width = 120;
        _magImage.Height = 96;
        _magImage.Stretch = Stretch.UniformToFill;
        _magImage.SnapsToDevicePixels = true;
        dock.Children.Add(_magLabel);
        dock.Children.Add(_magImage);
        _magnifier.Child = dock;
    }

    private void UpdateMagnifier(Point dip, (int X, int Y) virt)
    {
        if (!_showMagnifier || _session.IsAnnotating)
        {
            _magnifier.Visibility = Visibility.Collapsed;
            return;
        }

        int localX = virt.X - _frame.Monitor.Bounds.X;
        int localY = virt.Y - _frame.Monitor.Bounds.Y;
        try
        {
            BitmapSource? zoom = _frame.Buffer.TryCropZoom(localX, localY, srcHalf: 20, scale: 2);
            if (zoom is null)
            {
                if (_magFailHint is null)
                {
                    _magFailHint = "放大镜在这块屏不可用。";
                }

                _magnifier.Visibility = Visibility.Collapsed;
                return;
            }

            _magImage.Source = zoom;
            string label = localX + "," + localY;
            if (_frame.Buffer.TryGetPixel(localX, localY, out byte b, out byte g, out byte r, out _))
            {
                label = ColorPick.ToHex(r, g, b);
            }

            _magLabel.Text = label;
            double w = ActualWidth > 0 ? ActualWidth : Width;
            double h = ActualHeight > 0 ? ActualHeight : Height;
            double mx = dip.X + 16;
            double my = dip.Y + 16;
            if (mx + 120 > w)
            {
                mx = dip.X - 16 - 120;
            }

            if (my + 120 > h)
            {
                my = dip.Y - 16 - 120;
            }

            Canvas.SetLeft(_magnifier, Math.Max(0, mx));
            Canvas.SetTop(_magnifier, Math.Max(0, my));
            _magnifier.Visibility = Visibility.Visible;
        }
        catch
        {
            _magnifier.Visibility = Visibility.Collapsed;
        }
    }


}
