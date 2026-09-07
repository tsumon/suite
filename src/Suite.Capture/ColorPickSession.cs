using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;
using Suite.Capture.Grabbers;

namespace Suite.Capture;

/// <summary>F2 color pick: fullscreen crosshair + magnifier. INTERACTION-P2 §6.</summary>
public sealed class ColorPickSession : IDisposable
{
    private readonly List<Window> _windows = [];
    private readonly Action<string> _report;
    private bool _done;

    public ColorPickSession(Action<string> report)
    {
        _report = report;
    }

    public void Begin()
    {
        IReadOnlyList<MonitorCapture> frames;
        try
        {
            frames = ScreenCapturePipeline.CaptureAll();
        }
        catch
        {
            _report(ColorPick.Fail);
            return;
        }

        foreach (MonitorCapture frame in frames)
        {
            var win = new ColorPickOverlay(frame, OnPicked, OnCancel);
            _windows.Add(win);
            win.Closed += (_, _) =>
            {
                frame.Buffer.ReleasePixels();
            };
            win.Show();
        }

        if (_windows.Count > 0)
        {
            _windows[0].Activate();
        }
    }

    private void OnPicked(string hex)
    {
        if (_done)
        {
            return;
        }

        _done = true;
        if (ColorPick.TryCopyHex(hex, out string? error))
        {
            _report(string.Format(ColorPick.CopiedFmt, hex));
        }
        else
        {
            _report(error ?? ColorPick.Fail);
        }

        CloseAll();
    }

    private void OnCancel()
    {
        if (_done)
        {
            return;
        }

        _done = true;
        CloseAll();
    }

    private void CloseAll()
    {
        foreach (Window w in _windows.ToArray())
        {
            try
            {
                w.Close();
            }
            catch
            {
            }
        }

        _windows.Clear();
    }

    public void Dispose() => CloseAll();
}

internal sealed class ColorPickOverlay : Window
{
    private readonly MonitorCapture _frame;
    private readonly Action<string> _picked;
    private readonly Action _cancel;
    private readonly Image _magImage = new();
    private readonly TextBlock _magLabel = new();
    private readonly Border _mag = new();
    private readonly double _dipX;
    private readonly double _dipY;
    private BitmapSource? _source;
    private readonly Image _backdrop = new();

    public ColorPickOverlay(MonitorCapture frame, Action<string> picked, Action cancel)
    {
        _frame = frame;
        _picked = picked;
        _cancel = cancel;
        _dipX = 96.0 / Math.Max(1, (int)frame.Monitor.DpiX);
        _dipY = 96.0 / Math.Max(1, (int)frame.Monitor.DpiY);
        _source = frame.Buffer.ToBitmapSource();

        WindowStyle = WindowStyle.None;
        ResizeMode = ResizeMode.NoResize;
        ShowInTaskbar = false;
        Topmost = true;
        Cursor = Cursors.Cross;
        Background = Brushes.Black;
        Focusable = true;
        Title = "Suite 取色";
        Left = frame.Monitor.Bounds.X * _dipX;
        Top = frame.Monitor.Bounds.Y * _dipY;
        Width = frame.Monitor.Bounds.Width * _dipX;
        Height = frame.Monitor.Bounds.Height * _dipY;

        _backdrop.Source = _source;
        _backdrop.Stretch = Stretch.Fill;
        _backdrop.SnapsToDevicePixels = true;
        _mag.Width = 120;
        _mag.Height = 120;
        _mag.CornerRadius = new CornerRadius(2);
        _mag.BorderBrush = Brushes.White;
        _mag.BorderThickness = new Thickness(1);
        _mag.Background = Brushes.Black;
        _mag.IsHitTestVisible = false;
        var magStack = new DockPanel();
        _magLabel.Foreground = Brushes.White;
        _magLabel.FontSize = 11;
        _magLabel.FontFamily = new FontFamily("Consolas, Segoe UI");
        _magLabel.Padding = new Thickness(4, 2, 4, 2);
        _magLabel.HorizontalAlignment = HorizontalAlignment.Center;
        DockPanel.SetDock(_magLabel, Dock.Bottom);
        _magImage.Stretch = Stretch.UniformToFill;
        _magImage.Width = 120;
        _magImage.Height = 96;
        magStack.Children.Add(_magLabel);
        magStack.Children.Add(_magImage);
        _mag.Child = magStack;

        var canvas = new Canvas();
        canvas.Children.Add(_backdrop);
        Canvas.SetLeft(_backdrop, 0);
        Canvas.SetTop(_backdrop, 0);
        _backdrop.Width = Width;
        _backdrop.Height = Height;
        canvas.Children.Add(_mag);
        Content = canvas;

        MouseMove += OnMove;
        MouseLeftButtonUp += OnUp;
        MouseRightButtonUp += (_, _) => _cancel();
        KeyDown += (_, e) =>
        {
            if (e.Key == Key.Escape)
            {
                _cancel();
                e.Handled = true;
            }
        };
        Loaded += (_, _) =>
        {
            Activate();
            Keyboard.Focus(this);
        };
        Closed += (_, _) =>
        {
            _backdrop.Source = null;
            _magImage.Source = null;
            _source = null;
        };
    }

    private void OnMove(object sender, MouseEventArgs e)
    {
        Point dip = e.GetPosition(this);
        int lx = (int)Math.Round(dip.X / _dipX);
        int ly = (int)Math.Round(dip.Y / _dipY);
        UpdateMag(dip, lx, ly);
    }

    private void OnUp(object sender, MouseButtonEventArgs e)
    {
        Point dip = e.GetPosition(this);
        int lx = (int)Math.Round(dip.X / _dipX);
        int ly = (int)Math.Round(dip.Y / _dipY);
        if (_frame.Buffer.TryGetPixel(lx, ly, out byte b, out byte g, out byte r, out _))
        {
            _picked(ColorPick.ToHex(r, g, b));
            e.Handled = true;
            return;
        }

        _cancel();
    }

    private void UpdateMag(Point dip, int lx, int ly)
    {
        BitmapSource? zoom = _frame.Buffer.TryCropZoom(lx, ly, srcHalf: 20, scale: 2);
        if (zoom is not null)
        {
            _magImage.Source = zoom;
        }

        string hex = "#------";
        if (_frame.Buffer.TryGetPixel(lx, ly, out byte b, out byte g, out byte r, out _))
        {
            hex = ColorPick.ToHex(r, g, b);
        }

        _magLabel.Text = hex;
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

        Canvas.SetLeft(_mag, Math.Max(0, mx));
        Canvas.SetTop(_mag, Math.Max(0, my));
    }
}
