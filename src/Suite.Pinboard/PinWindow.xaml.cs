using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using Suite.Contracts;
using Suite.Platform;

namespace Suite.Pinboard;

public partial class PinWindow : Window
{
    public const double GlowPad = 22;
    private bool _clickThrough;
    private ThemeKind _theme = ThemeKind.Dark;
    private Storyboard? _hueStoryboard;

    // 彩虹色环（HSV 近似），动画沿环滚动
    private static readonly Color[] Rainbow =
    [
        Color.FromRgb(0xFF, 0x6A, 0x8A),
        Color.FromRgb(0xFF, 0xB8, 0x6A),
        Color.FromRgb(0xE8, 0xE0, 0x70),
        Color.FromRgb(0x5E, 0xDC, 0x9A),
        Color.FromRgb(0x5A, 0xB8, 0xFF),
        Color.FromRgb(0xA8, 0x78, 0xFF),
        Color.FromRgb(0xFF, 0x6A, 0x8A),
    ];

    public PinWindow(BitmapSource image, uint dpiX = 96, uint dpiY = 96)
    {
        InitializeComponent();
        if (image.CanFreeze && !image.IsFrozen)
        {
            image.Freeze();
        }

        Picture.Source = image;
        var (width, height) = DipConvert.Size(image.PixelWidth, image.PixelHeight, dpiX, dpiY);
        Picture.Width = Math.Max(1, width);
        Picture.Height = Math.Max(1, height);
        GlowFarRing.Width = Picture.Width;
        GlowFarRing.Height = Picture.Height;
        GlowNearRing.Width = Picture.Width;
        GlowNearRing.Height = Picture.Height;
        Zoom.ScaleX = 1;
        Zoom.ScaleY = 1;
        ApplyTheme(ThemeKind.Dark);
        Opacity = 1.0;
        MouseEnter += (_, _) => SetGlowStrength(hover: true);
        MouseLeave += (_, _) => SetGlowStrength(hover: false);
        Loaded += (_, _) =>
        {
            FadeGlowIn();
            StartHueCycle();
            Activate();
            Focus();
        };
        Closed += (_, _) =>
        {
            StopHueCycle();
            // Drop full BGRA BitmapSource so closed pins do not keep WorkingSet high.
            Picture.Source = null;
        };
        PreviewMouseDown += OnMiddleClick;
    }

    private void OnMiddleClick(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton != MouseButton.Middle)
        {
            return;
        }

        SetClickThrough(!_clickThrough);
        e.Handled = true;
    }

    public bool IsClickThrough => _clickThrough;

    public event EventHandler? ClickThroughChanged;

    public void PlaceContentAt(double left, double top)
    {
        Left = left - GlowPad;
        Top = top - GlowPad;
    }

    public void ApplyTheme(ThemeKind theme)
    {
        _theme = theme;
        // 彩虹光环：主题不再强行白/黑描边，只调透明度
        SetGlowStrength(IsMouseOver);
    }

    public void SetClickThrough(bool enabled)
    {
        _clickThrough = enabled;
        ClickThroughItem.IsChecked = enabled;
        IntPtr hwnd = new WindowInteropHelper(this).Handle;
        PinNative.SetClickThrough(hwnd, enabled);
        ClickThroughChanged?.Invoke(this, EventArgs.Empty);
    }

    private void FadeGlowIn()
    {
        GlowFarRing.BeginAnimation(OpacityProperty, Fade(FarOpacity(false)));
        GlowNearRing.BeginAnimation(OpacityProperty, Fade(NearOpacity(false)));
    }

    private static DoubleAnimation Fade(double to) => new(0, to, TimeSpan.FromMilliseconds(120))
    {
        FillBehavior = FillBehavior.HoldEnd,
    };

    private void SetGlowStrength(bool hover)
    {
        GlowFarRing.BeginAnimation(OpacityProperty, null);
        GlowNearRing.BeginAnimation(OpacityProperty, null);
        GlowFarRing.Opacity = FarOpacity(hover);
        GlowNearRing.Opacity = NearOpacity(hover);
    }

    private double FarOpacity(bool hover)
    {
        // Mid between neon original and soft pastel — stronger on light
        double rest = _theme == ThemeKind.Dark ? 0.32 : 0.48;
        double bump = _theme == ThemeKind.Dark ? 0.08 : 0.10;
        return hover ? Math.Min(1, rest + bump) : rest;
    }

    private double NearOpacity(bool hover)
    {
        double rest = _theme == ThemeKind.Dark ? 0.42 : 0.58;
        double bump = _theme == ThemeKind.Dark ? 0.08 : 0.10;
        return hover ? Math.Min(1, rest + bump) : rest;
    }

    private void StartHueCycle()
    {
        StopHueCycle();
        _hueStoryboard = new Storyboard { RepeatBehavior = RepeatBehavior.Forever };
        TimeSpan duration = TimeSpan.FromSeconds(6);
        GradientStop[] farStops = [Far0, Far1, Far2, Far3, Far4, Far5, Far6];
        GradientStop[] nearStops = [Near0, Near1, Near2, Near3, Near4, Near5];
        for (int i = 0; i < farStops.Length; i++)
        {
            _hueStoryboard.Children.Add(HueAnim(farStops[i], i, Rainbow.Length, duration));
        }

        for (int i = 0; i < nearStops.Length; i++)
        {
            _hueStoryboard.Children.Add(HueAnim(nearStops[i], i + 1, Rainbow.Length, duration));
        }

        _hueStoryboard.Begin();
    }

    private static ColorAnimationUsingKeyFrames HueAnim(GradientStop stop, int phase, int count, TimeSpan duration)
    {
        var anim = new ColorAnimationUsingKeyFrames
        {
            Duration = duration,
            RepeatBehavior = RepeatBehavior.Forever,
        };
        for (int k = 0; k <= count; k++)
        {
            Color c = Rainbow[(k + phase) % count];
            double t = k / (double)count;
            anim.KeyFrames.Add(new LinearColorKeyFrame(c, KeyTime.FromPercent(t)));
        }

        Storyboard.SetTarget(anim, stop);
        Storyboard.SetTargetProperty(anim, new PropertyPath(GradientStop.ColorProperty));
        return anim;
    }

    private void StopHueCycle()
    {
        _hueStoryboard?.Stop();
        _hueStoryboard = null;
    }

    private void OnDrag(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton != MouseButton.Left || _clickThrough)
        {
            return;
        }

        if (!ReferenceEquals(e.OriginalSource, Picture))
        {
            return;
        }

        DragMove();
    }

    private void OnWheel(object sender, MouseWheelEventArgs e)
    {
        if ((Keyboard.Modifiers & ModifierKeys.Alt) == ModifierKeys.Alt)
        {
            double next = Opacity + (e.Delta > 0 ? 0.08 : -0.08);
            Opacity = Math.Clamp(next, 0.15, 1.0);
            e.Handled = true;
            return;
        }

        Point pointer = e.GetPosition(Picture);
        double oldScale = Zoom.ScaleX;
        double factor = e.Delta > 0 ? 1.1 : 1.0 / 1.1;
        double scale = Math.Clamp(oldScale * factor, 0.15, 8.0);
        Zoom.ScaleX = scale;
        Zoom.ScaleY = scale;
        Left += pointer.X * (oldScale - scale);
        Top += pointer.Y * (oldScale - scale);
        e.Handled = true;
    }

    private void OnKey(object sender, KeyEventArgs e)
    {
        if (_clickThrough)
        {
            return;
        }

        if (e.Key is Key.Escape or Key.Delete)
        {
            Close();
            e.Handled = true;
        }
    }

    private void OnOpacityFull(object sender, RoutedEventArgs e) => Opacity = 1.0;

    private void OnOpacity75(object sender, RoutedEventArgs e) => Opacity = 0.75;

    private void OnOpacity50(object sender, RoutedEventArgs e) => Opacity = 0.50;

    private void OnOpacity25(object sender, RoutedEventArgs e) => Opacity = 0.25;

    private void OnToggleClickThrough(object sender, RoutedEventArgs e) => SetClickThrough(!_clickThrough);

    private void OnCloseClick(object sender, RoutedEventArgs e) => Close();
}
