using System.Globalization;
using Suite.Capture.Ocr;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;

namespace Suite.Capture;

public partial class AnnotationWindow : Window
{
    private static readonly Brush ToolbarBg = Freeze(Color.FromRgb(0xF2, 0xF2, 0xF2));
    private static readonly Brush ToolbarLine = Freeze(Color.FromRgb(0xC8, 0xC8, 0xC8));
    private static readonly Brush ToolbarHover = Freeze(Color.FromRgb(0xE8, 0xE8, 0xE8));
    private static readonly Brush ToolbarPressed = Freeze(Color.FromRgb(0xDD, 0xDD, 0xDD));
    private static readonly Brush SeparatorInk = Freeze(Color.FromRgb(0x7B, 0x7B, 0x7B));
    private static readonly Brush DotInk = Freeze(Color.FromRgb(0xFF, 0x00, 0x00));
    private static readonly Brush CommitInk = Freeze(Color.FromRgb(0x20, 0x80, 0xF0));
    private static readonly Brush AnchorFill = Freeze(Colors.White);
    private static readonly Brush ChipBg = Freeze(Color.FromArgb(0xCC, 0, 0, 0));
    private static readonly Brush DimBrush = Freeze(Color.FromArgb(0x6E, 0, 0, 0));

    private IReadOnlyList<MonitorCapture> _frames;
    private readonly MonitorInfo _monitor;
    private readonly IOcrService _ocr;
    private readonly Action<string>? _reportStatus;
    private bool _ocrBusy;
    private PixelBuffer? _cachedBake;
    private bool _bakeDirty = true;
    private readonly List<IAnnotationOp> _ops = [];
    private readonly List<IAnnotationOp> _redo = [];
    private readonly Dictionary<AnnotateTool, ToolButton> _buttons = [];
    private readonly Ellipse[] _anchors = new Ellipse[8];
    private readonly double _dipX;
    private readonly double _dipY;
    private PixelBuffer _original;
    private PixelRect _selection;
    private AnnotateTool? _tool;
    private bool _ellipse;
    private bool _dragging;
    private bool _resizing;
    private bool _moving;
    private Handle _handle = Handle.None;
    private Point _startDip;
    private List<(int X, int Y)>? _stroke;
    private Shape? _draft;
    private PixelRect _resizeOrigin;
    private PixelRect _moveOrigin;
    private (int X, int Y) _moveStartVirtual;

    private readonly Grid _imageHost = new();
    private readonly Image _preview = new();
    private readonly Canvas _draftLayer = new() { IsHitTestVisible = false };
    private readonly Canvas _inputLayer = new();
    private readonly TextBox _textEntry = new();
    private readonly Rectangle _border = new();
    private readonly Border _toolbar = new();
    private readonly Border _sizeChip = new();
    private readonly TextBlock _sizeText = new();
    private readonly Rectangle _dimTop = new();
    private readonly Rectangle _dimBottom = new();
    private readonly Rectangle _dimLeft = new();
    private readonly Rectangle _dimRight = new();
    private DateTime _armedUtc = DateTime.MaxValue;
    private ToolButton? _undoButton;
    private ToolButton? _redoButton;
    // Cached Measure results for drag/resize light layout (skip Measure every move).
    private double _cachedBarW = 120;
    private double _cachedChipW = 48;
    private double _cachedChipH = 18;
    private string? _cachedChipText;

    // Live move/resize: RenderTransform + rubber-band — avoid Image Measure/Arrange every mousemove.
    private readonly TranslateTransform _liveTx = new();
    private bool _liveTxAttached;
    private bool _renderHooked;
    private bool _dimsDirty;
    private PixelRect _rubberSelection;
    private double _laidSelL;
    private double _laidSelT;
    private double _laidSelW;
    private double _laidSelH;

    // Full-monitor frozen screenshot (Snipaste-like). Dim hole reveals live viewport during drag.
    private readonly Image _freezeImage = new();
    private BitmapSource? _freezeSource;
    private bool _liveViewport;

    internal AnnotationWindow(
        IReadOnlyList<MonitorCapture> frames,
        PixelRect selection,
        MonitorInfo monitor,
        IOcrService? ocr = null,
        Action<string>? reportStatus = null)
    {
        _frames = frames;
        _monitor = monitor;
        _ocr = ocr ?? new NullOcrService();
        _reportStatus = reportStatus;
        Focusable = true;
        Loaded += (_, _) => ArmKeyboardFocus();
        ContentRendered += (_, _) => ArmKeyboardFocus();
        Activated += (_, _) => ArmKeyboardFocus();
        _selection = selection;
        _original = RegionCropper.Crop(frames, selection);
        _dipX = 96.0 / Math.Max(1, (int)monitor.DpiX);
        _dipY = 96.0 / Math.Max(1, (int)monitor.DpiY);
        InitializeComponent();
        BuildChrome();
        Left = monitor.Bounds.X * _dipX;
        Top = monitor.Bounds.Y * _dipY;
        Width = Math.Max(1, monitor.Bounds.Width * _dipX);
        Height = Math.Max(1, monitor.Bounds.Height * _dipY);
        LayoutChrome();
        Loaded += (_, _) =>
        {
            _armedUtc = DateTime.UtcNow;
            LayoutChrome();
            Activate();
        };
    }

    public PixelBuffer? Committed { get; private set; }
    public bool PinRequested { get; private set; }
    public bool Accepted { get; private set; }
    public PixelRect Selection => _selection;
    public PixelBuffer Peek() => Bake();

    public event Action<PixelRect>? RegionChanged;
    public event Action? SaveClicked;
    public event Action<string>? StatusMessage;

    /// <summary>Drop full-screen capture frames, freeze/preview BitmapSources, and bake cache after annotate ends.</summary>
    public void ReleaseSessionBuffers()
    {
        foreach (MonitorCapture frame in _frames)
        {
            frame.Buffer.ReleasePixels();
        }

        _frames = Array.Empty<MonitorCapture>();
        _cachedBake?.ReleasePixels();
        _cachedBake = null;
        _original.ReleasePixels();
        _bakeDirty = true;

        // Full-monitor freeze + live-viewport preview hold large frozen BitmapSources — drop on close/cancel/commit.
        _freezeImage.Source = null;
        _freezeSource = null;
        _preview.Source = null;
        _draftLayer.Children.Clear();
        _liveViewport = false;
    }

    private enum Handle
    {
        None, N, S, E, W, NE, NW, SE, SW,
    }

    private void BuildChrome()
    {
        BuildFreezeBackdrop();
        _preview.Stretch = Stretch.Fill;
        _preview.SnapsToDevicePixels = true;
        RenderOptions.SetBitmapScalingMode(_preview, BitmapScalingMode.NearestNeighbor);
        _imageHost.Background = Brushes.Transparent;
        _imageHost.Children.Add(_preview);
        _imageHost.Children.Add(_draftLayer);
        _imageHost.Children.Add(_inputLayer);
        _imageHost.MouseLeftButtonDown += OnCanvasDown;
        _imageHost.MouseMove += OnCanvasMove;
        _imageHost.MouseLeftButtonUp += OnCanvasUp;
        _imageHost.MouseDown += OnImageMouseDown;
        RefreshHostCursor();

        _textEntry.Visibility = Visibility.Collapsed;
        _textEntry.FontSize = 16;
        _textEntry.MinWidth = 120;
        _textEntry.CaretBrush = Brushes.White;
        _textEntry.Foreground = new SolidColorBrush(Color.FromRgb(0xDC, 0x28, 0x28));
        _textEntry.Background = new SolidColorBrush(Color.FromArgb(0xAA, 0, 0, 0));
        _textEntry.BorderBrush = new SolidColorBrush(Color.FromRgb(0xDC, 0x28, 0x28));
        _textEntry.AcceptsReturn = false;
        _textEntry.KeyDown += OnTextKey;
        _inputLayer.Children.Add(_textEntry);

        foreach (Rectangle dim in new[] { _dimTop, _dimBottom, _dimLeft, _dimRight })
        {
            dim.Fill = DimBrush;
            dim.IsHitTestVisible = true;
            dim.MouseLeftButtonDown += OnDimClick;
        }

        _border.Stroke = CommitInk;
        _border.StrokeThickness = AnnotateToolbar.StrokePx;
        _border.StrokeDashArray = null; // Joe: solid blue, no dashed frame
        _border.Fill = Brushes.Transparent;
        _border.IsHitTestVisible = false;

        for (int i = 0; i < _anchors.Length; i++)
        {
            var dot = new Ellipse
            {
                Width = AnnotateToolbar.AnchorDiameter,
                Height = AnnotateToolbar.AnchorDiameter,
                Fill = AnchorFill,
                Stroke = CommitInk,
                StrokeThickness = 1.5,
            };
            dot.MouseLeftButtonDown += OnAnchorDown;
            _anchors[i] = dot;
        }

        _sizeText.Foreground = Brushes.White;
        _sizeText.FontFamily = new FontFamily("Segoe UI, Consolas, Microsoft YaHei UI");
        _sizeText.FontSize = 11;
        _sizeChip.Background = ChipBg;
        _sizeChip.CornerRadius = new CornerRadius(3);
        _sizeChip.Padding = new Thickness(6, 2, 6, 2);
        _sizeChip.Child = _sizeText;
        _sizeChip.IsHitTestVisible = false;

        BuildToolbar();

        // z-order: freeze → dims (hole) → bake preview → chrome
        Root.Children.Add(_freezeImage);
        Root.Children.Add(_dimTop);
        Root.Children.Add(_dimBottom);
        Root.Children.Add(_dimLeft);
        Root.Children.Add(_dimRight);
        Root.Children.Add(_imageHost);
        Root.Children.Add(_border);
        Root.Children.Add(_sizeChip);
        Root.Children.Add(_toolbar);
        foreach (Ellipse dot in _anchors)
        {
            Root.Children.Add(dot);
        }
    }

    private void BuildToolbar()
    {
        _toolbar.Background = ToolbarBg;
        _toolbar.BorderBrush = ToolbarLine;
        _toolbar.BorderThickness = new Thickness(1);
        _toolbar.CornerRadius = new CornerRadius(2);
        _toolbar.Height = AnnotateToolbar.Height;
        _toolbar.Padding = new Thickness(8, 2, 8, 2);
        var row = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
        foreach (AnnotateTool tool in AnnotateToolbar.Order)
        {
            if (tool is AnnotateTool.Undo or AnnotateTool.Close)
            {
                row.Children.Add(MakeSeparator());
            }

            ToolButton button = MakeButton(tool);
            _buttons[tool] = button;
            if (tool == AnnotateTool.Undo)
            {
                _undoButton = button;
            }

            if (tool == AnnotateTool.Redo)
            {
                _redoButton = button;
            }

            row.Children.Add(button);
        }

        _toolbar.Child = row;
        RefreshHistory();
    }

    private static FrameworkElement MakeSeparator()
    {
        return new Border
        {
            Width = AnnotateToolbar.SeparatorWidth,
            Height = AnnotateToolbar.Height,
            Child = new Rectangle
            {
                Width = 1,
                Height = 16,
                Fill = SeparatorInk,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            },
        };
    }

    private ToolButton MakeButton(AnnotateTool tool)
    {
        var button = new ToolButton(tool);
        button.Click += (_, _) => OnToolClick(tool);
        if (tool == AnnotateTool.Shape)
        {
            var menu = new ContextMenu();
            var rect = new MenuItem { Header = "矩形" };
            rect.Click += (_, _) =>
            {
                _ellipse = false;
                SelectTool(AnnotateTool.Shape);
            };
            var ell = new MenuItem { Header = "椭圆" };
            ell.Click += (_, _) =>
            {
                _ellipse = true;
                SelectTool(AnnotateTool.Shape);
            };
            menu.Items.Add(rect);
            menu.Items.Add(ell);
            button.ContextMenu = menu;
        }

        return button;
    }

    private void OnToolClick(AnnotateTool tool)
    {
        switch (tool)
        {
            case AnnotateTool.Undo:
                Undo();
                return;
            case AnnotateTool.Redo:
                Redo();
                return;
            case AnnotateTool.Close:
                TryCancel();
                return;
            case AnnotateTool.Pin:
                Finish(pin: true);
                return;
            case AnnotateTool.Save:
                SaveClicked?.Invoke();
                return;
            case AnnotateTool.Copy:
                Finish(pin: false);
                return;
            case AnnotateTool.Ocr:
                _ = RecognizeOcrAsync();
                return;
            case AnnotateTool.Pick:
                SelectTool(tool);
                return;
            case AnnotateTool.Shape:
                _ellipse = false;
                SelectTool(tool);
                return;
            default:
                SelectTool(tool);
                return;
        }
    }

    private void SelectTool(AnnotateTool tool)
    {
        _tool = tool;
        foreach (ToolButton button in _buttons.Values)
        {
            button.SetSelected(AnnotateToolbar.ShowsSelectedDot(button.Tool) && button.Tool == tool);
        }

        RefreshHostCursor();
    }

    private void RefreshHostCursor() =>
        _imageHost.Cursor = _tool is null ? Cursors.SizeAll : Cursors.Cross;

    private void BuildFreezeBackdrop()
    {
        MonitorCapture? frame = null;
        foreach (MonitorCapture candidate in _frames)
        {
            if (candidate.Monitor.Handle == _monitor.Handle
                || candidate.Monitor.Bounds.Equals(_monitor.Bounds))
            {
                frame = candidate;
                break;
            }
        }

        frame ??= _frames.Count > 0 ? _frames[0] : null;
        if (frame is null)
        {
            return;
        }

        _freezeSource = frame.Buffer.ToBitmapSource();
        _freezeImage.Source = _freezeSource;
        _freezeImage.Stretch = Stretch.Fill;
        _freezeImage.SnapsToDevicePixels = true;
        _freezeImage.IsHitTestVisible = false;
        RenderOptions.SetBitmapScalingMode(_freezeImage, BitmapScalingMode.NearestNeighbor);
        Canvas.SetLeft(_freezeImage, 0);
        Canvas.SetTop(_freezeImage, 0);
    }

    /// <summary>
    /// Snipaste-like live viewport: hide bake sticker so the frozen full-screen image
    /// shows through the dim hole at the current selection. No Bake during drag.
    /// </summary>
    private void EnterLiveViewport()
    {
        if (_liveViewport)
        {
            return;
        }

        _liveViewport = true;
        // Hide bake sticker entirely so the frozen full-screen image shows through the dim hole
        // at the live selection — same as Snipaste/WeChat (content follows position, not a sticker).
        _imageHost.Opacity = 0;
        _imageHost.IsHitTestVisible = false;
        _preview.Visibility = Visibility.Collapsed;
        _draftLayer.Visibility = Visibility.Collapsed;
        _inputLayer.Visibility = Visibility.Collapsed;
    }

    private void ExitLiveViewport()
    {
        if (!_liveViewport)
        {
            return;
        }

        _liveViewport = false;
        _imageHost.Opacity = 1;
        _imageHost.IsHitTestVisible = true;
        _preview.Visibility = Visibility.Visible;
        _draftLayer.Visibility = Visibility.Visible;
        _inputLayer.Visibility = Visibility.Visible;
    }

    private void LayoutChrome(bool rebakePreview = true)
    {
        // Full layout path only — never call this every mousemove during _moving/_resizing.
        DetachLiveTransform();
        if (rebakePreview)
        {
            ExitLiveViewport();
        }

        double winW = ActualWidth > 1 ? ActualWidth : Width;
        double winH = ActualHeight > 1 ? ActualHeight : Height;
        _freezeImage.Width = Math.Max(1, winW);
        _freezeImage.Height = Math.Max(1, winH);

        double selL = (_selection.X - _monitor.Bounds.X) * _dipX;
        double selT = (_selection.Y - _monitor.Bounds.Y) * _dipY;
        double selW = Math.Max(1, _selection.Width * _dipX);
        double selH = Math.Max(1, _selection.Height * _dipY);
        double workL = (_monitor.WorkArea.X - _monitor.Bounds.X) * _dipX;
        double workT = (_monitor.WorkArea.Y - _monitor.Bounds.Y) * _dipY;
        double workR = (_monitor.WorkArea.Right - _monitor.Bounds.X) * _dipX;
        double workB = (_monitor.WorkArea.Bottom - _monitor.Bounds.Y) * _dipY;

        if (rebakePreview)
        {
            _preview.Source = Bake().ToBitmapSource();
        }

        _preview.Width = selW;
        _preview.Height = selH;
        _imageHost.Width = selW;
        _imageHost.Height = selH;
        _draftLayer.Width = selW;
        _draftLayer.Height = selH;
        Canvas.SetLeft(_imageHost, selL);
        Canvas.SetTop(_imageHost, selT);

        Canvas.SetLeft(_border, selL);
        Canvas.SetTop(_border, selT);
        _border.Width = selW;
        _border.Height = selH;

        _laidSelL = selL;
        _laidSelT = selT;
        _laidSelW = selW;
        _laidSelH = selH;

        UpdateDims(selL, selT, selW, selH);
        PlaceAnchors(selL, selT, selW, selH);

        _sizeText.Text = CaptureUx.SizeChip(_selection.Width, _selection.Height);
        MeasureChromeSizes(force: rebakePreview);
        PlaceToolbarAndChip(selL, selT, selW, selH, workL, workT, workR, workB);
    }

    private void MeasureChromeSizes(bool force)
    {
        if (force || !string.Equals(_cachedChipText, _sizeText.Text, StringComparison.Ordinal))
        {
            _sizeChip.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
            _cachedChipW = Math.Max(8, _sizeChip.DesiredSize.Width);
            _cachedChipH = Math.Max(8, _sizeChip.DesiredSize.Height);
            _cachedChipText = _sizeText.Text;
        }

        if (force)
        {
            _toolbar.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
            _cachedBarW = Math.Max(120, _toolbar.DesiredSize.Width);
        }
    }

    private void PlaceToolbarAndChip(
        double selL, double selT, double selW, double selH,
        double workL, double workT, double workR, double workB)
    {
        double chipW = _cachedChipW;
        double chipH = _cachedChipH;
        double barW = _cachedBarW;
        double barH = AnnotateToolbar.Height;

        AnnotateToolbar.Placement place = AnnotateToolbar.Place(
            selL, selT, selW, selH, barW, barH, workL, workT, workR, workB);
        Canvas.SetLeft(_toolbar, place.Left);
        Canvas.SetTop(_toolbar, place.Top);

        double chipX = selL + ((selW - chipW) / 2);
        double chipY = selT - chipH - 2;
        if (place.Above)
        {
            chipY = selT + 4;
        }

        if (chipY < 0)
        {
            chipY = selT + 4;
        }

        Canvas.SetLeft(_sizeChip, chipX);
        Canvas.SetTop(_sizeChip, chipY);
    }

    private void PlaceAnchors(double selL, double selT, double selW, double selH)
    {
        PlaceAnchor(0, selL, selT); // NW
        PlaceAnchor(1, selL + (selW / 2), selT); // N
        PlaceAnchor(2, selL + selW, selT); // NE
        PlaceAnchor(3, selL + selW, selT + (selH / 2)); // E
        PlaceAnchor(4, selL + selW, selT + selH); // SE
        PlaceAnchor(5, selL + (selW / 2), selT + selH); // S
        PlaceAnchor(6, selL, selT + selH); // SW
        PlaceAnchor(7, selL, selT + (selH / 2)); // W
        _anchors[0].Tag = Handle.NW;
        _anchors[1].Tag = Handle.N;
        _anchors[2].Tag = Handle.NE;
        _anchors[3].Tag = Handle.E;
        _anchors[4].Tag = Handle.SE;
        _anchors[5].Tag = Handle.S;
        _anchors[6].Tag = Handle.SW;
        _anchors[7].Tag = Handle.W;
    }

    private void UpdateDims(double selL, double selT, double selW, double selH)
    {
        double winW = ActualWidth > 1 ? ActualWidth : Width;
        double winH = ActualHeight > 1 ? ActualHeight : Height;
        SetDim(_dimTop, 0, 0, winW, Math.Max(0, selT));
        SetDim(_dimBottom, 0, selT + selH, winW, Math.Max(0, winH - selT - selH));
        SetDim(_dimLeft, 0, selT, Math.Max(0, selL), selH);
        SetDim(_dimRight, selL + selW, selT, Math.Max(0, winW - selL - selW), selH);
    }

    private void AttachLiveTransform()
    {
        if (_liveTxAttached)
        {
            return;
        }

        _liveTx.X = 0;
        _liveTx.Y = 0;
        // Do not transform _imageHost: bake sticker is hidden; freeze under dim hole is the viewport.
        _border.RenderTransform = _liveTx;
        _sizeChip.RenderTransform = _liveTx;
        _toolbar.RenderTransform = _liveTx;
        foreach (Ellipse dot in _anchors)
        {
            dot.RenderTransform = _liveTx;
        }

        _liveTxAttached = true;
    }

    private void DetachLiveTransform()
    {
        if (!_liveTxAttached)
        {
            _liveTx.X = 0;
            _liveTx.Y = 0;
            return;
        }

        _liveTx.X = 0;
        _liveTx.Y = 0;
        _imageHost.RenderTransform = null;
        _border.RenderTransform = null;
        _sizeChip.RenderTransform = null;
        _toolbar.RenderTransform = null;
        foreach (Ellipse dot in _anchors)
        {
            dot.RenderTransform = null;
        }

        _liveTxAttached = false;
    }

    private void HookLiveRendering()
    {
        if (_renderHooked)
        {
            return;
        }

        CompositionTarget.Rendering += OnLiveRendering;
        _renderHooked = true;
    }

    private void UnhookLiveRendering()
    {
        if (!_renderHooked)
        {
            return;
        }

        CompositionTarget.Rendering -= OnLiveRendering;
        _renderHooked = false;
        _dimsDirty = false;
    }

    private void OnLiveRendering(object? sender, EventArgs e)
    {
        if (_moving && _dimsDirty)
        {
            _dimsDirty = false;
            // Dims follow selection hole; transform already moved chrome — use live selection DIP.
            double selL = (_selection.X - _monitor.Bounds.X) * _dipX;
            double selT = (_selection.Y - _monitor.Bounds.Y) * _dipY;
            UpdateDims(selL, selT, _laidSelW, _laidSelH);
        }

        if (!_moving && !_resizing)
        {
            UnhookLiveRendering();
        }
    }

    /// <summary>
    /// Move path: translate chrome via RenderTransform; hide bake sticker so freeze shows
    /// through the dim hole at the new position (Snipaste real-time background).
    /// </summary>
    private void ApplyLiveMove()
    {
        EnterLiveViewport();
        AttachLiveTransform();
        _liveTx.X = (_selection.X - _moveOrigin.X) * _dipX;
        _liveTx.Y = (_selection.Y - _moveOrigin.Y) * _dipY;
        _dimsDirty = true;
        HookLiveRendering();
    }

    /// <summary>
    /// Resize path: rubber-band chrome + dim hole. Bake sticker hidden; freeze viewport
    /// shows through the hole. Optional CroppedBitmap kept in sync for host size.
    /// </summary>
    private void ApplyRubberBand(PixelRect sel)
    {
        EnterLiveViewport();
        double selL = (sel.X - _monitor.Bounds.X) * _dipX;
        double selT = (sel.Y - _monitor.Bounds.Y) * _dipY;
        double selW = Math.Max(1, sel.Width * _dipX);
        double selH = Math.Max(1, sel.Height * _dipY);
        double workL = (_monitor.WorkArea.X - _monitor.Bounds.X) * _dipX;
        double workT = (_monitor.WorkArea.Y - _monitor.Bounds.Y) * _dipY;
        double workR = (_monitor.WorkArea.Right - _monitor.Bounds.X) * _dipX;
        double workB = (_monitor.WorkArea.Bottom - _monitor.Bounds.Y) * _dipY;

        Canvas.SetLeft(_border, selL);
        Canvas.SetTop(_border, selT);
        _border.Width = selW;
        _border.Height = selH;

        PlaceAnchors(selL, selT, selW, selH);
        UpdateDims(selL, selT, selW, selH);

        _sizeText.Text = CaptureUx.SizeChip(sel.Width, sel.Height);
        // No Measure during drag — estimate chip width from glyph count if text changed.
        if (!string.Equals(_cachedChipText, _sizeText.Text, StringComparison.Ordinal))
        {
            _cachedChipW = Math.Max(48, 12 + (_sizeText.Text.Length * 7));
            _cachedChipText = _sizeText.Text;
        }

        PlaceToolbarAndChip(selL, selT, selW, selH, workL, workT, workR, workB);
    }

    private void BeginRubberResize()
    {
        DetachLiveTransform();
        EnterLiveViewport();
        _rubberSelection = _selection;
    }

    private static void SetDim(Rectangle rect, double x, double y, double w, double h)
    {
        Canvas.SetLeft(rect, x);
        Canvas.SetTop(rect, y);
        rect.Width = Math.Max(0, w);
        rect.Height = Math.Max(0, h);
    }

    private void OnDimClick(object sender, MouseButtonEventArgs e)
    {
        if (TryCancel())
        {
            e.Handled = true;
        }
    }

    private bool IsArmed =>
        (DateTime.UtcNow - _armedUtc).TotalMilliseconds >= CaptureUx.AnnotateArmMs;

    private bool TryCancel()
    {
        if (!IsArmed)
        {
            return false;
        }

        Close();
        return true;
    }

    private void PlaceAnchor(int index, double x, double y)
    {
        double d = AnnotateToolbar.AnchorDiameter;
        Canvas.SetLeft(_anchors[index], x - (d / 2));
        Canvas.SetTop(_anchors[index], y - (d / 2));
    }

    private void OnPreviewMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Middle)
        {
            Finish(pin: true);
            e.Handled = true;
        }
    }

    private void OnRightUp(object sender, MouseButtonEventArgs e)
    {
        if (_textEntry.Visibility == Visibility.Visible)
        {
            return;
        }

        if (e.OriginalSource is MenuItem || e.OriginalSource is ContextMenu
            || _buttons.Values.Any(b => b.ContextMenu is { IsOpen: true })
            || IsUnder(_toolbar, e.OriginalSource as DependencyObject))
        {
            return;
        }

        if (TryCancel())
        {
            e.Handled = true;
        }
    }

    private static bool IsUnder(DependencyObject root, DependencyObject? node)
    {
        while (node is not null)
        {
            if (ReferenceEquals(node, root))
            {
                return true;
            }

            node = VisualTreeHelper.GetParent(node);
        }

        return false;
    }

    private void ArmKeyboardFocus()
    {
        try
        {
            Activate();
            Focus();
            Keyboard.Focus(this);
        }
        catch
        {
        }
    }

    private void OnPreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (_textEntry.Visibility == Visibility.Visible)
        {
            return;
        }

        Key key = e.Key == Key.System ? e.SystemKey : e.Key;
        bool ctrl = (Keyboard.Modifiers & ModifierKeys.Control) == ModifierKeys.Control
            || Keyboard.IsKeyDown(Key.LeftCtrl)
            || Keyboard.IsKeyDown(Key.RightCtrl);

        if (key == Key.Escape)
        {
            Close();
            e.Handled = true;
            return;
        }

        // Enter / Ctrl+C = same as Copy toolbar button (clipboard + exit, no pin).
        if (key == Key.Enter || (key == Key.C && ctrl))
        {
            Finish(pin: false);
            e.Handled = true;
            return;
        }

        if (key == Key.Z && ctrl)
        {
            Undo();
            e.Handled = true;
            return;
        }

        if (key == Key.Y && ctrl)
        {
            Redo();
            e.Handled = true;
        }
    }

    private void OnImageMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ClickCount >= 2 && e.ChangedButton == MouseButton.Left)
        {
            Finish(pin: false);
            e.Handled = true;
        }
    }

    private void OnAnchorDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not Ellipse ellipse || ellipse.Tag is not Handle handle)
        {
            return;
        }

        _resizing = true;
        _handle = handle;
        _resizeOrigin = _selection;
        BeginRubberResize();
        CaptureMouse();
        e.Handled = true;
    }

    private void OnCanvasDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ClickCount >= 2)
        {
            return;
        }

        Point pos = e.GetPosition(_preview);
        if (_tool == AnnotateTool.Text)
        {
            ShowTextBox(pos);
            e.Handled = true;
            return;
        }

        if (_tool == AnnotateTool.Pick)
        {
            PickColorAt(pos);
            e.Handled = true;
            return;
        }

        if (_tool is null)
        {
            // 无绘制工具：在预览上拖动 = 平移选区（保持 W×H）
            _moving = true;
            _moveOrigin = _selection;
            _moveStartVirtual = ToVirtual(e.GetPosition(this));
            EnterLiveViewport();
            AttachLiveTransform();
            HookLiveRendering();
            CaptureMouse();
            e.Handled = true;
            return;
        }

        _dragging = true;
        _startDip = pos;
        _stroke = [(ToPxX(pos.X), ToPxY(pos.Y))];
        _imageHost.CaptureMouse();
        _draftLayer.Children.Clear();
        if (_tool is AnnotateTool.Pencil or AnnotateTool.Marker or AnnotateTool.Eraser or AnnotateTool.Curve)
        {
            var line = new Polyline
            {
                Stroke = _tool == AnnotateTool.Marker
                    ? new SolidColorBrush(Color.FromArgb(0x70, 0xDC, 0x28, 0x28))
                    : Brushes.OrangeRed,
                StrokeThickness = _tool == AnnotateTool.Marker ? 12 : _tool == AnnotateTool.Eraser ? 16 : 2,
                StrokeLineJoin = PenLineJoin.Round,
                StrokeStartLineCap = PenLineCap.Round,
                StrokeEndLineCap = PenLineCap.Round,
            };
            line.Points.Add(pos);
            _draft = line;
        }
        else
        {
            _draft = new System.Windows.Shapes.Rectangle
            {
                Stroke = Brushes.OrangeRed,
                StrokeThickness = 2,
                Fill = Brushes.Transparent,
            };
            Canvas.SetLeft(_draft, pos.X);
            Canvas.SetTop(_draft, pos.Y);
        }

        _draftLayer.Children.Add(_draft);
        e.Handled = true;
    }

    private void OnCanvasMove(object sender, MouseEventArgs e)
    {
        if (_resizing)
        {
            (int x, int y) = ToVirtual(e.GetPosition(this));
            PixelRect next = ResizeRect(_resizeOrigin, _handle, x, y);
            if (next.Width >= CaptureUx.MinCommitPx && next.Height >= CaptureUx.MinCommitPx)
            {
                _selection = next;
                _rubberSelection = next;
                // Border/anchors only — keep 1:1 with cursor; Image deferred to CommitResize.
                ApplyRubberBand(next);
            }

            return;
        }

        if (_moving)
        {
            (int x, int y) = ToVirtual(e.GetPosition(this));
            int dx = x - _moveStartVirtual.X;
            int dy = y - _moveStartVirtual.Y;
            _selection = CaptureUx.MoveRect(_moveOrigin, dx, dy, ClampBounds());
            // Transform only — Image Width/Height/Source untouched until CommitMove.
            ApplyLiveMove();
            return;
        }

        if (!_dragging || _draft is null)
        {
            return;
        }

        Point pos = e.GetPosition(_preview);
        _stroke?.Add((ToPxX(pos.X), ToPxY(pos.Y)));
        if (_draft is Polyline poly)
        {
            poly.Points.Add(pos);
            if (_tool == AnnotateTool.Curve && poly.Points.Count > 1)
            {
                poly.Points[1] = pos;
                while (poly.Points.Count > 2)
                {
                    poly.Points.RemoveAt(2);
                }
            }

            return;
        }

        double x0 = Math.Min(_startDip.X, pos.X);
        double y0 = Math.Min(_startDip.Y, pos.Y);
        _draft.Width = Math.Abs(pos.X - _startDip.X);
        _draft.Height = Math.Abs(pos.Y - _startDip.Y);
        Canvas.SetLeft(_draft, x0);
        Canvas.SetTop(_draft, y0);
    }

    protected override void OnMouseMove(MouseEventArgs e)
    {
        base.OnMouseMove(e);
        if (_resizing || _moving)
        {
            OnCanvasMove(this, e);
        }
    }

    protected override void OnMouseLeftButtonUp(MouseButtonEventArgs e)
    {
        base.OnMouseLeftButtonUp(e);
        if (_resizing)
        {
            _resizing = false;
            _handle = Handle.None;
            UnhookLiveRendering();
            if (IsMouseCaptured)
            {
                ReleaseMouseCapture();
            }

            CommitResize();
            e.Handled = true;
            return;
        }

        if (_moving)
        {
            _moving = false;
            UnhookLiveRendering();
            if (IsMouseCaptured)
            {
                ReleaseMouseCapture();
            }

            CommitMove();
            e.Handled = true;
        }
    }

    private void OnCanvasUp(object sender, MouseButtonEventArgs e)
    {
        if (_resizing || _moving)
        {
            return;
        }

        if (!_dragging)
        {
            return;
        }

        _dragging = false;
        if (_imageHost.IsMouseCaptured)
        {
            _imageHost.ReleaseMouseCapture();
        }

        Point pos = e.GetPosition(_preview);
        _draftLayer.Children.Clear();
        _draft = null;
        IReadOnlyList<(int X, int Y)> stroke = _stroke ?? [];
        _stroke = null;

        if (_tool is AnnotateTool.Pencil or AnnotateTool.Marker or AnnotateTool.Eraser)
        {
            if (stroke.Count == 0)
            {
                return;
            }

            if (_tool == AnnotateTool.Eraser)
            {
                Push(new EraseOp(stroke, 18));
            }
            else if (_tool == AnnotateTool.Marker)
            {
                Push(new StrokeOp(stroke, 14, 40, 40, 220, 110));
            }
            else
            {
                Push(new StrokeOp(stroke, 2, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, 255));
            }

            return;
        }

        if (_tool == AnnotateTool.Curve)
        {
            Push(new StrokeOp(
                [(ToPxX(_startDip.X), ToPxY(_startDip.Y)), (ToPxX(pos.X), ToPxY(pos.Y))],
                3,
                PixelDraw.StrokeB,
                PixelDraw.StrokeG,
                PixelDraw.StrokeR,
                255));
            return;
        }

        PixelRect rect = PixelRect.FromCorners(ToPxX(_startDip.X), ToPxY(_startDip.Y), ToPxX(pos.X), ToPxY(pos.Y));
        if (rect.Width < 2 && rect.Height < 2)
        {
            return;
        }

        if (_tool == AnnotateTool.Mosaic)
        {
            Push(new MosaicOp(rect));
            return;
        }

        if (_tool == AnnotateTool.Shape && _ellipse)
        {
            Push(new EllipseOp(rect));
            return;
        }

        if (_tool == AnnotateTool.Shape)
        {
            Push(new RectOp(rect));
        }
    }

    private void CommitResize()
    {
        if (_selection.Width < CaptureUx.MinCommitPx || _selection.Height < CaptureUx.MinCommitPx)
        {
            return;
        }

        PixelBuffer nextCrop = RegionCropper.Crop(_frames, _selection);
        _original.ReleasePixels();
        _original = nextCrop;
        _ops.Clear();
        _redo.Clear();
        _bakeDirty = true;
        RefreshHistory();
        LayoutChrome();
        RegionChanged?.Invoke(_selection);
    }

    private void CommitMove()
    {
        if (_selection.Width < CaptureUx.MinCommitPx || _selection.Height < CaptureUx.MinCommitPx)
        {
            return;
        }

        // v1：平移后重裁，清空标注栈（坐标相对旧裁剪区）
        PixelBuffer nextCrop = RegionCropper.Crop(_frames, _selection);
        _original.ReleasePixels();
        _original = nextCrop;
        _ops.Clear();
        _redo.Clear();
        _bakeDirty = true;
        RefreshHistory();
        LayoutChrome();
        RegionChanged?.Invoke(_selection);
    }

    private PixelRect ClampBounds()
    {
        var list = new List<PixelRect>(_frames.Count);
        foreach (MonitorCapture frame in _frames)
        {
            list.Add(frame.Monitor.Bounds);
        }

        return CaptureUx.UnionMonitorBounds(list, _monitor.Bounds);
    }

    private static PixelRect ResizeRect(PixelRect start, Handle handle, int x, int y)
    {
        int l = start.X;
        int t = start.Y;
        int r = start.Right;
        int b = start.Bottom;
        switch (handle)
        {
            case Handle.N:
                t = y;
                break;
            case Handle.S:
                b = y;
                break;
            case Handle.E:
                r = x;
                break;
            case Handle.W:
                l = x;
                break;
            case Handle.NE:
                return ScaleCorner(start, start.X, start.Bottom, x, y);
            case Handle.NW:
                return ScaleCorner(start, start.Right, start.Bottom, x, y);
            case Handle.SE:
                return ScaleCorner(start, start.X, start.Y, x, y);
            case Handle.SW:
                return ScaleCorner(start, start.Right, start.Y, x, y);
        }

        return PixelRect.FromCorners(l, t, r, b);
    }

    private static PixelRect ScaleCorner(PixelRect start, int fixedX, int fixedY, int x, int y)
    {
        if (start.Width <= 0 || start.Height <= 0)
        {
            return PixelRect.FromCorners(fixedX, fixedY, x, y);
        }

        double sx = (x - fixedX) / (double)start.Width;
        double sy = (y - fixedY) / (double)start.Height;
        double scale = Math.Abs(sx) > Math.Abs(sy) ? sx : sy;
        if (Math.Abs(scale) < 0.01)
        {
            scale = 0.01;
        }

        int w = Math.Max(CaptureUx.MinCommitPx, (int)Math.Round(start.Width * Math.Abs(scale)));
        int h = Math.Max(CaptureUx.MinCommitPx, (int)Math.Round(start.Height * Math.Abs(scale)));
        int nx = x < fixedX ? fixedX - w : fixedX;
        int ny = y < fixedY ? fixedY - h : fixedY;
        return new PixelRect(nx, ny, w, h);
    }

    private void ShowTextBox(Point dip)
    {
        Canvas.SetLeft(_textEntry, dip.X);
        Canvas.SetTop(_textEntry, dip.Y);
        _textEntry.Tag = dip;
        _textEntry.Text = "";
        _textEntry.Visibility = Visibility.Visible;
        _textEntry.Focus();
    }

    private void OnTextKey(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            _textEntry.Visibility = Visibility.Collapsed;
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Enter)
        {
            return;
        }

        CommitText();
        e.Handled = true;
    }

    private void CommitText()
    {
        if (_textEntry.Visibility != Visibility.Visible)
        {
            return;
        }

        string text = _textEntry.Text.Trim();
        _textEntry.Visibility = Visibility.Collapsed;
        if (text.Length > 0 && _textEntry.Tag is Point dip)
        {
            Push(new TextOp(ToPxX(dip.X), ToPxY(dip.Y), text));
        }
    }

    private void Push(IAnnotationOp op)
    {
        _ops.Add(op);
        _redo.Clear();
        _bakeDirty = true;
        RefreshPreview();
        RefreshHistory();
    }

    private void Undo()
    {
        if (_ops.Count == 0)
        {
            return;
        }

        IAnnotationOp op = _ops[^1];
        _ops.RemoveAt(_ops.Count - 1);
        _redo.Add(op);
        _bakeDirty = true;
        RefreshPreview();
        RefreshHistory();
    }

    private void Redo()
    {
        if (_redo.Count == 0)
        {
            return;
        }

        IAnnotationOp op = _redo[^1];
        _redo.RemoveAt(_redo.Count - 1);
        _ops.Add(op);
        _bakeDirty = true;
        RefreshPreview();
        RefreshHistory();
    }

    private void RefreshHistory()
    {
        SetHistory(_undoButton, _ops.Count > 0);
        SetHistory(_redoButton, _redo.Count > 0);
    }

    private static void SetHistory(ToolButton? button, bool enabled)
    {
        if (button is null)
        {
            return;
        }

        button.IsEnabled = enabled;
        button.Opacity = enabled ? 1 : 0.4;
    }

    private void RefreshPreview() => _preview.Source = Bake().ToBitmapSource();

    private void Finish(bool pin)
    {
        CommitText();
        Committed = Bake().Clone();
        PinRequested = pin;
        Accepted = true;
        Close();
    }

    private PixelBuffer Bake()
    {
        if (!_bakeDirty && _cachedBake is not null && _cachedBake.Width > 0)
        {
            return _cachedBake;
        }

        _cachedBake?.ReleasePixels();
        PixelBuffer result = _original.Clone();
        foreach (IAnnotationOp op in _ops)
        {
            op.Apply(result, _original);
        }

        _cachedBake = result;
        _bakeDirty = false;
        return result;
    }

    private async Task RecognizeOcrAsync()
    {
        if (_ocrBusy)
        {
            return;
        }

        _ocrBusy = true;
        ToolButton? ocrBtn = _buttons.GetValueOrDefault(AnnotateTool.Ocr);
        if (ocrBtn is not null)
        {
            ocrBtn.IsEnabled = false;
            ocrBtn.Opacity = 0.4;
        }

        try
        {
            PixelBuffer image = Bake().Clone();
            try
            {
                OcrResult result = await _ocr.RecognizeAsync(image).ConfigureAwait(true);
                if (result.Succeeded)
                {
                    try
                    {
                        System.Windows.Clipboard.SetText(result.Text ?? "");
                        NotifyStatus(WindowsOcrService.Copied);
                    }
                    catch
                    {
                        NotifyStatus(WindowsOcrService.Failed);
                    }
                }
                else if (!string.IsNullOrEmpty(result.Error))
                {
                    NotifyStatus(result.Error!);
                }
            }
            finally
            {
                image.ReleasePixels();
            }
        }
        catch (OperationCanceledException)
        {
            // 静默：取消本次识别，栏仍在
        }
        catch
        {
            NotifyStatus(WindowsOcrService.Failed);
        }
        finally
        {
            _ocrBusy = false;
            if (ocrBtn is not null)
            {
                ocrBtn.IsEnabled = true;
                ocrBtn.Opacity = 1;
            }
        }
    }


    private void PickColorAt(Point dip)
    {
        int x = ToPxX(dip.X);
        int y = ToPxY(dip.Y);
        PixelBuffer bake = Bake();
        if (!bake.TryGetPixel(x, y, out byte b, out byte g, out byte r, out _))
        {
            NotifyStatus(ColorPick.Fail);
            return;
        }

        string hex = ColorPick.ToHex(r, g, b);
        if (ColorPick.TryCopyHex(hex, out string? error))
        {
            NotifyStatus(string.Format(ColorPick.CopiedFmt, hex));
        }
        else
        {
            NotifyStatus(error ?? ColorPick.Fail);
        }
    }

    private void NotifyStatus(string message)
    {
        if (string.IsNullOrEmpty(message))
        {
            return;
        }

        if (_reportStatus is not null)
        {
            _reportStatus(message);
        }
        else
        {
            StatusMessage?.Invoke(message);
        }
    }

    private (int X, int Y) ToVirtual(Point windowPos) =>
        (
            _monitor.Bounds.X + (int)Math.Round(windowPos.X / _dipX),
            _monitor.Bounds.Y + (int)Math.Round(windowPos.Y / _dipY));

    private int ToPxX(double dip)
    {
        double width = _preview.ActualWidth > 1 ? _preview.ActualWidth : Math.Max(1, _original.Width * _dipX);
        return (int)Math.Round(dip * _original.Width / width);
    }

    private int ToPxY(double dip)
    {
        double height = _preview.ActualHeight > 1 ? _preview.ActualHeight : Math.Max(1, _original.Height * _dipY);
        return (int)Math.Round(dip * _original.Height / height);
    }

    private static SolidColorBrush Freeze(Color color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }

    private sealed class ToolButton : Button
    {
        public ToolButton(AnnotateTool tool)
        {
            Tool = tool;
            Width = AnnotateToolbar.Cell;
            Height = AnnotateToolbar.Cell;
            Focusable = false;
            ToolTip = AnnotateToolbar.Tooltip(tool);
            Background = Brushes.Transparent;
            BorderThickness = new Thickness(0);
            Padding = new Thickness(0);
            var grid = new Grid();
            var glyph = AnnotateIcons.Glyph(tool);
            glyph.HorizontalAlignment = HorizontalAlignment.Center;
            glyph.VerticalAlignment = VerticalAlignment.Center;
            glyph.IsHitTestVisible = false;
            Dot = new Ellipse
            {
                Width = AnnotateToolbar.SelectedDot,
                Height = AnnotateToolbar.SelectedDot,
                Fill = DotInk,
                HorizontalAlignment = HorizontalAlignment.Right,
                VerticalAlignment = VerticalAlignment.Top,
                Margin = new Thickness(0, 4, 6, 0),
                Visibility = Visibility.Collapsed,
                IsHitTestVisible = false,
            };
            grid.Children.Add(glyph);
            grid.Children.Add(Dot);
            Content = grid;
            Template = BuildTemplate();
        }

        public AnnotateTool Tool { get; }
        public Ellipse Dot { get; }

        public void SetSelected(bool selected) =>
            Dot.Visibility = selected ? Visibility.Visible : Visibility.Collapsed;

        private static ControlTemplate BuildTemplate()
        {
            var template = new ControlTemplate(typeof(Button));
            var border = new FrameworkElementFactory(typeof(Border));
            border.Name = "Bd";
            border.SetValue(Border.BackgroundProperty, Brushes.Transparent);
            border.SetValue(Border.SnapsToDevicePixelsProperty, true);
            var presenter = new FrameworkElementFactory(typeof(ContentPresenter));
            presenter.SetValue(HorizontalAlignmentProperty, HorizontalAlignment.Center);
            presenter.SetValue(VerticalAlignmentProperty, VerticalAlignment.Center);
            border.AppendChild(presenter);
            template.VisualTree = border;
            var hover = new Trigger { Property = IsMouseOverProperty, Value = true };
            hover.Setters.Add(new Setter(Border.BackgroundProperty, ToolbarHover) { TargetName = "Bd" });
            var pressed = new Trigger { Property = IsPressedProperty, Value = true };
            pressed.Setters.Add(new Setter(Border.BackgroundProperty, ToolbarPressed) { TargetName = "Bd" });
            template.Triggers.Add(hover);
            template.Triggers.Add(pressed);
            return template;
        }
    }

    private interface IAnnotationOp
    {
        void Apply(PixelBuffer buffer, PixelBuffer original);
    }

    private sealed class RectOp(PixelRect rect) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) => PixelDraw.Rectangle(buffer, rect);
    }

    private sealed class EllipseOp(PixelRect rect) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) => PixelDraw.Ellipse(buffer, rect);
    }

    private sealed class MosaicOp(PixelRect rect) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) => PixelDraw.Mosaic(buffer, rect);
    }

    private sealed class StrokeOp(IReadOnlyList<(int X, int Y)> points, int thickness, byte b, byte g, byte r, byte a)
        : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.Polyline(buffer, points, thickness, b, g, r, a);
    }

    private sealed class EraseOp(IReadOnlyList<(int X, int Y)> points, int thickness) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.RestoreStroke(buffer, original, points, thickness);
    }

    private sealed class TextOp(int x, int y, string text) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original)
        {
            var visual = new DrawingVisual();
            var formatted = new FormattedText(
                text,
                CultureInfo.CurrentUICulture,
                FlowDirection.LeftToRight,
                new Typeface("Segoe UI"),
                18,
                new SolidColorBrush(Color.FromRgb(PixelDraw.StrokeR, PixelDraw.StrokeG, PixelDraw.StrokeB)),
                1.0);
            using (DrawingContext dc = visual.RenderOpen())
            {
                dc.DrawImage(buffer.ToBitmapSource(), new Rect(0, 0, buffer.Width, buffer.Height));
                dc.DrawText(formatted, new Point(x, y));
            }

            var rtb = new RenderTargetBitmap(buffer.Width, buffer.Height, 96, 96, PixelFormats.Pbgra32);
            rtb.Render(visual);
            rtb.Freeze();
            PixelBuffer baked = PixelBuffer.FromBitmapSource(rtb);
            Buffer.BlockCopy(baked.Bgra, 0, buffer.Bgra, 0, Math.Min(baked.Bgra.Length, buffer.Bgra.Length));
        }
    }
}
